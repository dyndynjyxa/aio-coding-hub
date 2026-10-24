//! Connection ownership and bounded, single-use continuation recovery metadata.

use super::protocol::{HistoryDigest, Owner, TURN_STATE_HEADER};
use super::upstream::UpstreamConnection;
use crate::gateway::proxy::GatewayFailure;
use crate::shared::mutex_ext::MutexExt;
use axum::http::HeaderMap;
use rand::RngCore;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};

const RECOVERY_TTL: Duration = Duration::from_secs(30);
const OWNER_IDLE_TTL: Duration = Duration::from_secs(30 * 60);
const MAX_RECOVERIES: usize = 128;
// shortcut: retain 128 prior windows per turn; revisit if real turns exceed this compaction count.
const MAX_PREVIOUS_CONTEXT_WINDOWS: usize = 128;
const WS_COOLDOWN: Duration = Duration::from_secs(60);
pub(in crate::gateway) const MAX_CONNECTIONS: usize = 20;
// shortcut: fixed reservations are not measured heap usage; measure peaks before increasing capacity again.
const RAW_BUFFER_RESERVATION: u32 = 128 * 1024 * 1024;
const RAW_BUFFER_BUDGET: usize = MAX_CONNECTIONS * RAW_BUFFER_RESERVATION as usize;

pub(in crate::gateway) struct Runtime {
    enabled: AtomicBool,
    epoch: AtomicU64,
    pub(in crate::gateway) changed: watch::Sender<u64>,
    pub(in crate::gateway) shutdown: watch::Sender<bool>,
    connections: Arc<Semaphore>,
    raw_buffers: Arc<Semaphore>,
    cooldowns: Mutex<HashMap<String, WsCooldown>>,
    force_http_sessions: Mutex<HashMap<String, Instant>>,
    owners: Mutex<HashMap<Owner, OwnerRecord>>,
}

impl Runtime {
    pub(in crate::gateway) fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            epoch: AtomicU64::new(0),
            changed: watch::channel(0).0,
            shutdown: watch::channel(false).0,
            connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            raw_buffers: Arc::new(Semaphore::new(RAW_BUFFER_BUDGET)),
            cooldowns: Mutex::new(HashMap::new()),
            force_http_sessions: Mutex::new(HashMap::new()),
            owners: Mutex::new(HashMap::new()),
        }
    }

    pub(in crate::gateway) fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire) && !*self.shutdown.borrow()
    }

    pub(in crate::gateway) fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    pub(in crate::gateway) fn set_enabled(&self, enabled: bool) {
        if self.enabled.swap(enabled, Ordering::AcqRel) != enabled {
            self.invalidate();
        }
    }

    pub(in crate::gateway) fn invalidate(&self) {
        self.owners.lock_or_recover().clear();
        self.cooldowns.lock_or_recover().clear();
        self.force_http_sessions.lock_or_recover().clear();
        let epoch = self.epoch.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
        self.changed.send_replace(epoch);
    }

    pub(in crate::gateway) fn stop(&self) {
        self.enabled.store(false, Ordering::Release);
        self.invalidate();
        self.shutdown.send_replace(true);
    }

    pub(in crate::gateway) fn connection(
        self: &Arc<Self>,
    ) -> Result<Arc<Connection>, &'static str> {
        if !self.enabled() {
            return Err("Responses WebSocket is disabled");
        }
        let permit = self
            .connections
            .clone()
            .try_acquire_owned()
            .map_err(|_| "Responses WebSocket connection limit reached")?;
        let buffers = self.reserve_raw_buffers()?;
        Ok(Arc::new(Connection {
            runtime: self.clone(),
            _permit: Some(permit),
            _buffers: buffers,
            epoch: self.epoch(),
            continuation: Mutex::new(None),
            upstream: Mutex::new(None),
            prewarm: Mutex::new(None),
        }))
    }

    fn reserve_raw_buffers(&self) -> Result<OwnedSemaphorePermit, &'static str> {
        self.raw_buffers
            .clone()
            .try_acquire_many_owned(RAW_BUFFER_RESERVATION)
            .map_err(|_| "Responses WebSocket raw buffer budget exhausted")
    }

    pub(in crate::gateway) fn cooling(&self, key: &str) -> bool {
        self.cooldowns
            .lock_or_recover()
            .get(key)
            .is_some_and(|entry| entry.until > Instant::now() || entry.probing)
    }

    /// An expired cooldown permits exactly one live probe; healthy endpoints need
    /// no probe lease. A canceled probe releases its slot without claiming health.
    pub(in crate::gateway) fn try_ws_probe(
        self: &Arc<Self>,
        key: &str,
    ) -> Result<Option<WsProbe>, ()> {
        let mut entries = self.cooldowns.lock_or_recover();
        let Some(entry) = entries.get_mut(key) else {
            return Ok(None);
        };
        if entry.until > Instant::now() || entry.probing {
            return Err(());
        }
        entry.probing = true;
        Ok(Some(WsProbe {
            runtime: self.clone(),
            key: key.to_owned(),
            until: entry.until,
        }))
    }

    pub(in crate::gateway) fn cool(&self, key: String) {
        let now = Instant::now();
        let mut entries = self.cooldowns.lock_or_recover();
        if !entries.contains_key(&key) && entries.len() >= MAX_RECOVERIES {
            let expired = entries
                .iter()
                .filter(|(_, entry)| !entry.probing && entry.until <= now)
                .min_by_key(|(_, entry)| entry.until)
                .map(|(key, _)| key.clone());
            if let Some(expired) = expired {
                entries.remove(&expired);
            }
        }
        if entries.len() < MAX_RECOVERIES || entries.contains_key(&key) {
            entries.insert(
                key,
                WsCooldown {
                    until: now + WS_COOLDOWN,
                    probing: false,
                },
            );
        }
    }

    pub(in crate::gateway) fn force_http(&self, session_key: &str) {
        if session_key.is_empty() || session_key.len() > 512 {
            return;
        }
        let mut entries = self.force_http_sessions.lock_or_recover();
        entries.retain(|_, until| *until > Instant::now());
        if entries.len() < MAX_RECOVERIES || entries.contains_key(session_key) {
            entries.insert(session_key.to_owned(), Instant::now() + RECOVERY_TTL);
        }
    }

    pub(in crate::gateway) fn prefers_http(&self, session_key: &str) -> bool {
        let mut entries = self.force_http_sessions.lock_or_recover();
        entries.retain(|_, until| *until > Instant::now());
        entries.contains_key(session_key)
    }

    pub(in crate::gateway) fn issue_nonce(&self, owner: &Owner) -> Result<String, &'static str> {
        if !self.enabled() {
            return Err("Responses WebSocket is disabled");
        }
        let mut records = self.owners.lock_or_recover();
        prune_owners(&mut records);
        if let Some(record) = records.get(owner) {
            if record.active_generation.is_some()
                || record.pending.is_some()
                || record.completed.is_none()
            {
                return Err("Responses generation owner is already in use");
            }
            // Local compaction opens another client session within the same turn.
            return Ok(record.nonce.clone());
        }
        let nonce = new_nonce();
        self.insert_owner(&mut records, owner, nonce.clone())?;
        Ok(nonce)
    }

    fn insert_owner(
        &self,
        records: &mut HashMap<Owner, OwnerRecord>,
        owner: &Owner,
        nonce: String,
    ) -> Result<(), &'static str> {
        if records.len() >= MAX_RECOVERIES {
            let oldest = records
                .iter()
                .filter(|(_, record)| {
                    record.active_generation.is_none() && record.pending.is_none()
                })
                .min_by_key(|(_, record)| record.expires)
                .map(|(owner, _)| owner.clone());
            if let Some(oldest) = oldest {
                records.remove(&oldest);
            } else {
                return Err("Responses recovery owner capacity reached");
            }
        }
        records.insert(
            owner.clone(),
            OwnerRecord {
                nonce,
                epoch: self.epoch(),
                active_generation: None,
                http_only: false,
                completed: None,
                previous_context_windows: HashSet::new(),
                pending: None,
                failure: None,
                expires: Instant::now() + OWNER_IDLE_TTL,
            },
        );
        Ok(())
    }

    /// Admit one generation globally for this owner, including across sockets.
    /// Call exactly once before sending the local nonce or dispatching upstream.
    pub(in crate::gateway) fn begin_generation(
        &self,
        request: &RequestState,
    ) -> Result<(), &'static str> {
        if !self.enabled() || request.connection.epoch != self.epoch() {
            return Err("Responses WebSocket is unavailable");
        }
        let generation = request.generation.lock_or_recover();
        let Some(identity) = &generation.identity else {
            return Ok(());
        };
        let mut records = self.owners.lock_or_recover();
        prune_owners(&mut records);
        let record = records
            .get_mut(&identity.owner)
            .ok_or("unknown Responses owner nonce")?;
        if !self.enabled()
            || record.epoch != self.epoch()
            || record.nonce != identity.nonce
            || record.active_generation.is_some()
            || (record.http_only && request.client_ws)
        {
            return Err("Responses generation owner is unavailable");
        }
        if generation.recovered {
            if !record
                .pending
                .as_ref()
                .is_some_and(|pending| pending.consumed && pending.expected == generation.expected)
            {
                return Err("Responses recovery was not claimed");
            }
        } else if record.pending.is_some() {
            // A fresh generation carries the client's authoritative history and no
            // inherited budget, so it need not extend the completed one: Codex retries
            // a stream it never saw finish and normalizes its history between requests.
            return Err("Responses generation conflicts with a pending recovery");
        }
        record.active_generation = Some(Arc::downgrade(&request.generation));
        record.http_only |= !request.client_ws || generation.budget.http_only;
        record.expires = Instant::now() + OWNER_IDLE_TTL;
        Ok(())
    }

    /// Every end releases the turn: Codex cancels a stream to steer or merge
    /// sub-agent input, and retries failed streams, with the same turn nonce.
    /// Only success advances the completed history; a suspended generation is no
    /// longer active here, so its unclaimed recovery grant survives.
    pub(in crate::gateway) fn finish_generation(&self, request: &RequestState, completed: bool) {
        let generation = request.generation.lock_or_recover();
        let Some(identity) = &generation.identity else {
            return;
        };
        let mut records = self.owners.lock_or_recover();
        let Some(record) = records.get_mut(&identity.owner) else {
            return;
        };
        if record.nonce != identity.nonce || record.epoch != self.epoch() {
            return;
        }
        let identity = Arc::downgrade(&request.generation);
        if !record
            .active_generation
            .as_ref()
            .is_some_and(|active| active.ptr_eq(&identity))
        {
            return;
        }
        record.active_generation = None;
        // An active generation's grant, if any, was consumed by this generation.
        record.pending = None;
        record.expires = Instant::now() + OWNER_IDLE_TTL;
        if completed {
            record.completed = Some(generation.expected.clone());
            record.failure = None;
            record.http_only |= !request.client_ws || generation.budget.http_only;
        }
    }

    pub(in crate::gateway) fn remember_failure(
        &self,
        request: &RequestState,
        failure: &GatewayFailure,
    ) {
        let Some(failure) = failure.unavailable_summary() else {
            return;
        };
        let generation = request.generation.lock_or_recover();
        let Some(identity) = &generation.identity else {
            return;
        };
        if generation.committed
            || !generation.expected.is_recoverable()
            || !generation.input.is_recoverable()
            || !generation.properties.is_recoverable()
        {
            return;
        }
        let mut records = self.owners.lock_or_recover();
        let Some(record) = records.get_mut(&identity.owner) else {
            return;
        };
        if record.epoch != self.epoch()
            || record.nonce != identity.nonce
            || !record
                .active_generation
                .as_ref()
                .is_some_and(|active| active.ptr_eq(&Arc::downgrade(&request.generation)))
            || record
                .pending
                .as_ref()
                .is_some_and(|pending| !pending.consumed)
        {
            return;
        }
        // Replay only until the announced recovery time; without one a retry is simply
        // routed again, which costs no upstream call while providers stay unavailable.
        record.failure = failure
            .retry_after_seconds
            .filter(|seconds| *seconds > 0)
            .and_then(|seconds| Instant::now().checked_add(Duration::from_secs(seconds)))
            .map(|retry_at| KnownFailure {
                retry_at,
                failure,
                input: generation.input.clone(),
                expected: generation.expected.clone(),
                properties: generation.properties.clone(),
                previous: generation.previous.clone(),
            });
    }

    /// Read an ended result before validating socket continuation or claiming recovery resources.
    pub(in crate::gateway) fn known_failure(
        &self,
        owner: &Owner,
        nonce: Option<&str>,
        input: &[Value],
        previous: Option<&str>,
        properties: &HistoryDigest,
    ) -> Result<Option<GatewayFailure>, &'static str> {
        let Some(nonce) = nonce else {
            return Ok(None);
        };
        let mut records = self.owners.lock_or_recover();
        prune_owners(&mut records);
        let Some(record) = records.get(owner) else {
            return Ok(None);
        };
        let Some(known) = &record.failure else {
            return Ok(None);
        };
        if !self.enabled()
            || record.epoch != self.epoch()
            || record.nonce != nonce
            || record.active_generation.is_some()
        {
            return Err("context recovery ownership mismatch");
        }
        let input = HistoryDigest::from_items(input);
        let original = previous == known.previous.as_deref() && input == known.input;
        let full = previous.is_none() && input == known.expected;
        if !input.is_recoverable() || *properties != known.properties || !(original || full) {
            // Not a retry of the failed request (e.g. steering was merged): route it normally.
            return Ok(None);
        }
        let mut failure = known.failure.clone();
        failure.retry_after_seconds = Some(
            known
                .retry_at
                .saturating_duration_since(Instant::now())
                .as_secs(),
        );
        Ok(Some(failure))
    }

    /// Circuit resets and cleared unavailability make remembered failures stale.
    pub(in crate::gateway) fn clear_failures(&self) {
        for record in self.owners.lock_or_recover().values_mut() {
            record.failure = None;
        }
    }

    pub(in crate::gateway) fn suspend(&self, request: &RequestState) -> Result<(), &'static str> {
        if !self.enabled() {
            return Err("Responses WebSocket was disabled");
        }
        let generation = request.generation.lock_or_recover();
        let identity = generation
            .identity
            .as_ref()
            .ok_or("context recovery requires an identified owner")?;
        if generation.recovered
            || generation.committed
            || generation.expected.count == 0
            || !generation.expected.is_recoverable()
            || !generation.properties.is_recoverable()
        {
            return Err("context recovery is not safe for this generation");
        }
        let mut records = self.owners.lock_or_recover();
        prune_owners(&mut records);
        let record = records
            .get_mut(&identity.owner)
            .ok_or("unknown Responses owner nonce")?;
        if record.nonce != identity.nonce
            || record.epoch != self.epoch()
            || record.active_generation.is_none()
            || record.pending.is_some()
            || !record
                .active_generation
                .as_ref()
                .is_some_and(|active| active.ptr_eq(&Arc::downgrade(&request.generation)))
        {
            return Err("context recovery owner is unavailable");
        }
        record.active_generation = None;
        record.pending = Some(Pending {
            expected: generation.expected.clone(),
            properties: generation.properties.clone(),
            budget: generation.budget.clone(),
            expires: Instant::now() + RECOVERY_TTL,
            from_trace: generation.trace_id.clone(),
            consumed: false,
        });
        record.expires = Instant::now() + RECOVERY_TTL;
        Ok(())
    }

    pub(in crate::gateway) fn claim(
        &self,
        owner: &Owner,
        nonce: Option<&str>,
        input: &[Value],
        properties: &HistoryDigest,
    ) -> Result<Option<Recovered>, &'static str> {
        self.claim_for_transport(owner, nonce, input, properties, true)
    }

    fn claim_for_transport(
        &self,
        owner: &Owner,
        nonce: Option<&str>,
        input: &[Value],
        properties: &HistoryDigest,
        client_ws: bool,
    ) -> Result<Option<Recovered>, &'static str> {
        let mut records = self.owners.lock_or_recover();
        prune_owners(&mut records);
        if !records.contains_key(owner) {
            let previous = records
                .iter()
                .find(|(previous, record)| {
                    let later_window = match (
                        previous.window.rsplit_once(':'),
                        owner.window.rsplit_once(':'),
                    ) {
                        (Some((old_thread, old)), Some((new_thread, new)))
                            if old_thread == previous.thread && new_thread == owner.thread =>
                        {
                            old.parse::<u64>()
                                .ok()
                                .zip(new.parse::<u64>().ok())
                                .is_some_and(|(old, new)| new > old)
                        }
                        _ => false,
                    };
                    self.enabled()
                        && record.epoch == self.epoch()
                        && nonce == Some(record.nonce.as_str())
                        && previous.session == owner.session
                        && previous.thread == owner.thread
                        && (previous.window == owner.window || later_window)
                        && previous.turn == owner.turn
                        && previous.context_window != owner.context_window
                        && !record
                            .previous_context_windows
                            .contains(&owner.context_window)
                        && record.active_generation.is_none()
                        && record.pending.is_none()
                        && record.failure.is_none()
                        && record.completed.is_some()
                        && (!client_ws || !record.http_only)
                })
                .map(|(previous, _)| previous.clone());
            if let Some(previous) = previous {
                if records[&previous].previous_context_windows.len() >= MAX_PREVIOUS_CONTEXT_WINDOWS
                {
                    return Err("Responses context window capacity reached");
                }
                // Codex keeps its first turn nonce after compaction; this starts new history,
                // while a pending or failed generation must keep its original recovery budget.
                let mut record = records.remove(&previous).expect("matched Responses owner");
                record
                    .previous_context_windows
                    .insert(previous.context_window);
                record.completed = None;
                records.insert(owner.clone(), record);
                return Ok(None);
            }
        }
        let Some(record) = records.get_mut(owner) else {
            let Some(nonce) = nonce.filter(|nonce| is_local_nonce(nonce)) else {
                return Ok(None);
            };
            let tracked = records.iter().any(|(known, record)| {
                record.nonce == nonce
                    || (known.session == owner.session
                        && known.thread == owner.thread
                        && known.turn == owner.turn)
            });
            if !self.enabled() || tracked {
                return Err("unknown or expired Responses owner nonce");
            }
            // Codex keeps a turn's first nonce for the whole turn. After a gateway restart,
            // settings change or idle expiry nothing tracks the turn any more, so it continues
            // under that nonce as a fresh owner; no recovery grant survives the lost record.
            self.insert_owner(&mut records, owner, nonce.to_owned())?;
            return Ok(None);
        };
        if nonce.is_none() && record.active_generation.is_none() && record.pending.is_none() {
            return Ok(None);
        }
        if !self.enabled()
            || record.epoch != self.epoch()
            || record.active_generation.is_some()
            || nonce != Some(record.nonce.as_str())
            || (client_ws && record.http_only)
        {
            return Err("context recovery ownership mismatch");
        }
        // Recovery is best effort: a request the gateway cannot match to its grant is
        // still the client's authoritative history, so it runs as a fresh generation.
        let Some(pending) = record.pending.as_mut() else {
            return Ok(None);
        };
        let expected = HistoryDigest::from_items(input);
        let exact = expected == pending.expected && pending.properties == *properties;
        if pending.consumed || (!exact && expected.count <= pending.expected.count) {
            return Err("context recovery history or constraints mismatch");
        }
        if !exact {
            // A longer resend supersedes the suspended request, e.g. Codex merged steering
            // or sub-agent input into it; stale or partial replays keep the grant intact.
            record.pending = None;
            return Ok(None);
        }
        pending.consumed = true;
        Ok(Some(Recovered {
            budget: pending.budget.clone(),
            from_trace: pending.from_trace.clone(),
        }))
    }

    /// Only locally issued tokens opt HTTP requests into recovery ownership.
    /// The caller must strip this recognized token from both headers and body
    /// before dispatch; ordinary HTTP requests are intentionally untouched here.
    pub(in crate::gateway) fn prepare_http_recovery(
        self: &Arc<Self>,
        headers: &HeaderMap,
        body: &Value,
        forced: Option<i64>,
    ) -> Result<PreparedRequest, &'static str> {
        let Some(nonce) = recovery_nonce(headers, body)? else {
            return Ok(PreparedRequest::Dispatch(None));
        };
        let body_owner = body
            .pointer("/client_metadata/x-codex-turn-metadata")
            .map(|value| {
                value
                    .as_str()
                    .and_then(Owner::parse)
                    .ok_or("invalid Codex turn metadata")
            })
            .transpose()?;
        let header_owner = headers
            .get("x-codex-turn-metadata")
            .map(|value| {
                value
                    .to_str()
                    .ok()
                    .and_then(Owner::parse)
                    .ok_or("invalid Codex turn metadata")
            })
            .transpose()?;
        if body_owner.is_some() && header_owner.is_some() && body_owner != header_owner {
            return Err("conflicting Codex turn metadata");
        }
        let owner = body_owner
            .or(header_owner)
            .ok_or("missing Codex turn metadata for recovery")?;
        let input = body
            .get("input")
            .and_then(Value::as_array)
            .ok_or("missing full recovery input")?;
        let properties = request_properties(body, forced);
        if let Some(failure) = self.known_failure(
            &owner,
            Some(nonce),
            input,
            body.get("previous_response_id").and_then(Value::as_str),
            &properties,
        )? {
            return Ok(PreparedRequest::Failure(failure));
        }
        if body
            .get("previous_response_id")
            .is_some_and(|value| !value.is_null())
        {
            return Err("HTTP context recovery requires full input");
        }
        // Reserve before claiming: local pressure must not consume a recovery grant.
        let buffers = self.reserve_raw_buffers()?;
        let recovered = self.claim_for_transport(&owner, Some(nonce), input, &properties, false)?;
        let (mut budget, from_trace) = recovered.map_or_else(
            || (Budget::default(), None),
            |record| (record.budget, Some(record.from_trace)),
        );
        budget.http_only = true;
        let request = RequestState {
            connection: Arc::new(Connection {
                runtime: self.clone(),
                _permit: None,
                _buffers: buffers,
                epoch: self.epoch(),
                continuation: Mutex::new(None),
                upstream: Mutex::new(None),
                prewarm: Mutex::new(None),
            }),
            client_ws: false,
            generation: Arc::new(Mutex::new(Generation {
                identity: Some(RecoveryIdentity {
                    owner,
                    nonce: nonce.to_owned(),
                }),
                expected: HistoryDigest::from_items(input),
                input: HistoryDigest::from_items(input),
                properties,
                previous: None,
                committed: false,
                terminal: false,
                incomplete: false,
                failed: false,
                recovered: from_trace.is_some(),
                from_trace,
                trace_id: String::new(),
                budget,
                upstream_ws: false,
            })),
        };
        self.begin_generation(&request)?;
        Ok(PreparedRequest::Dispatch(Some(request)))
    }
}

struct WsCooldown {
    until: Instant,
    probing: bool,
}

pub(in crate::gateway) struct WsProbe {
    runtime: Arc<Runtime>,
    key: String,
    until: Instant,
}

impl WsProbe {
    pub(in crate::gateway) fn succeeded(self) {
        let mut entries = self.runtime.cooldowns.lock_or_recover();
        if entries
            .get(&self.key)
            .is_some_and(|entry| entry.until == self.until)
        {
            entries.remove(&self.key);
        }
    }
}

impl Drop for WsProbe {
    fn drop(&mut self) {
        let mut entries = self.runtime.cooldowns.lock_or_recover();
        if let Some(entry) = entries
            .get_mut(&self.key)
            .filter(|entry| entry.until == self.until)
        {
            entry.probing = false;
        }
    }
}

struct OwnerRecord {
    nonce: String,
    epoch: u64,
    active_generation: Option<Weak<Mutex<Generation>>>,
    http_only: bool,
    completed: Option<HistoryDigest>,
    previous_context_windows: HashSet<String>,
    pending: Option<Pending>,
    failure: Option<KnownFailure>,
    expires: Instant,
}

fn prune_owners(records: &mut HashMap<Owner, OwnerRecord>) {
    let now = Instant::now();
    for record in records.values_mut() {
        if record
            .active_generation
            .as_ref()
            .is_some_and(|active| active.strong_count() == 0)
        {
            record.active_generation = None;
            record.pending = None;
            record.expires = now + OWNER_IDLE_TTL;
        }
        if record.active_generation.is_none()
            && record
                .pending
                .as_ref()
                .is_some_and(|pending| pending.expires <= now)
        {
            record.pending = None;
            record.expires = now + OWNER_IDLE_TTL;
        }
        if record
            .failure
            .as_ref()
            .is_some_and(|known| known.retry_at <= now)
        {
            record.failure = None;
        }
    }
    records.retain(|_, record| record.active_generation.is_some() || record.expires > now);
}

pub(in crate::gateway) struct Connection {
    pub(in crate::gateway) runtime: Arc<Runtime>,
    _permit: Option<OwnedSemaphorePermit>,
    _buffers: OwnedSemaphorePermit,
    pub(in crate::gateway) epoch: u64,
    pub(in crate::gateway) continuation: Mutex<Option<Continuation>>,
    pub(in crate::gateway) upstream: Mutex<Option<ReusableSocket>>,
    pub(in crate::gateway) prewarm: Mutex<Option<Prewarm>>,
}

pub(in crate::gateway) struct Prewarm {
    pub(in crate::gateway) response_id: String,
    pub(in crate::gateway) provider_id: i64,
    pub(in crate::gateway) history: HistoryDigest,
}

pub(in crate::gateway) struct ReusableSocket {
    pub(in crate::gateway) key: String,
    pub(in crate::gateway) provider_id: i64,
    pub(in crate::gateway) connection: UpstreamConnection,
    pub(in crate::gateway) turn_state: Option<String>,
}

#[derive(Clone)]
pub(in crate::gateway) struct RecoveryIdentity {
    pub(in crate::gateway) owner: Owner,
    pub(in crate::gateway) nonce: String,
}

pub(in crate::gateway) struct Continuation {
    pub(in crate::gateway) identity: Option<RecoveryIdentity>,
    pub(in crate::gateway) response_id: String,
    pub(in crate::gateway) provider_id: i64,
    pub(in crate::gateway) upstream_ws: bool,
    pub(in crate::gateway) history: HistoryDigest,
}

#[derive(Clone)]
pub(in crate::gateway) struct RequestState {
    pub(in crate::gateway) connection: Arc<Connection>,
    pub(in crate::gateway) generation: Arc<Mutex<Generation>>,
    pub(in crate::gateway) client_ws: bool,
}

pub(in crate::gateway) struct GenerationLease(pub(in crate::gateway) Option<RequestState>);

impl Drop for GenerationLease {
    fn drop(&mut self) {
        if let Some(request) = &self.0 {
            let completed = {
                let generation = request.generation.lock_or_recover();
                generation.terminal && !generation.failed
            };
            request
                .connection
                .runtime
                .finish_generation(request, completed);
        }
    }
}

pub(in crate::gateway) struct Generation {
    pub(in crate::gateway) identity: Option<RecoveryIdentity>,
    pub(in crate::gateway) expected: HistoryDigest,
    pub(in crate::gateway) input: HistoryDigest,
    pub(in crate::gateway) properties: HistoryDigest,
    pub(in crate::gateway) previous: Option<String>,
    pub(in crate::gateway) committed: bool,
    pub(in crate::gateway) terminal: bool,
    pub(in crate::gateway) incomplete: bool,
    pub(in crate::gateway) failed: bool,
    pub(in crate::gateway) recovered: bool,
    pub(in crate::gateway) from_trace: Option<String>,
    pub(in crate::gateway) trace_id: String,
    pub(in crate::gateway) budget: Budget,
    pub(in crate::gateway) upstream_ws: bool,
}

pub(in crate::gateway) enum PreparedRequest {
    Dispatch(Option<RequestState>),
    Failure(GatewayFailure),
}

#[cfg(test)]
impl PreparedRequest {
    fn unwrap_dispatch(self) -> Option<RequestState> {
        match self {
            Self::Dispatch(request) => request,
            Self::Failure(_) => panic!("expected generation dispatch"),
        }
    }
}

struct KnownFailure {
    failure: GatewayFailure,
    retry_at: Instant,
    input: HistoryDigest,
    expected: HistoryDigest,
    properties: HistoryDigest,
    previous: Option<String>,
}

#[derive(Clone, Default)]
pub(in crate::gateway) struct Budget {
    pub(in crate::gateway) providers: Vec<i64>,
    pub(in crate::gateway) provider_id: Option<i64>,
    pub(in crate::gateway) retry_index: u32,
    pub(in crate::gateway) tried_ws: HashSet<i64>,
    pub(in crate::gateway) http_provider_ids: HashSet<i64>,
    pub(in crate::gateway) visited_providers: HashSet<i64>,
    pub(in crate::gateway) failed_providers: HashSet<i64>,
    pub(in crate::gateway) deadline: Option<Instant>,
    pub(in crate::gateway) http_only: bool,
}

struct Pending {
    expected: HistoryDigest,
    properties: HistoryDigest,
    budget: Budget,
    expires: Instant,
    from_trace: String,
    consumed: bool,
}

pub(in crate::gateway) struct Recovered {
    pub(in crate::gateway) budget: Budget,
    pub(in crate::gateway) from_trace: String,
}

fn new_nonce() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!(
        "aio-ws-{}",
        bytes
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect::<String>()
    )
}

pub(in crate::gateway) fn recovery_nonce<'a>(
    headers: &'a HeaderMap,
    body: &'a Value,
) -> Result<Option<&'a str>, &'static str> {
    let header_value = headers.get(TURN_STATE_HEADER);
    let body_value = body.pointer("/client_metadata/x-codex-turn-state");
    let header_nonce = header_value.and_then(|value| value.to_str().ok());
    let body_nonce = body_value.and_then(Value::as_str);
    let nonce = header_nonce
        .filter(|value| is_local_nonce(value))
        .or_else(|| body_nonce.filter(|value| is_local_nonce(value)));
    if let Some(nonce) = nonce {
        if (header_value.is_some() && header_nonce != Some(nonce))
            || (body_value.is_some() && body_nonce != Some(nonce))
        {
            return Err("conflicting Responses owner nonce");
        }
    }
    Ok(nonce)
}

fn is_local_nonce(value: &str) -> bool {
    value.starts_with("aio-ws-")
}

pub(in crate::gateway) fn request_properties(
    body: &Value,
    forced_provider: Option<i64>,
) -> HistoryDigest {
    let mut properties = body.clone();
    if let Some(object) = properties.as_object_mut() {
        for field in [
            "input",
            "previous_response_id",
            "type",
            "stream",
            "generate",
            "client_metadata",
            "stream_options",
        ] {
            object.remove(field);
        }
        object.insert(
            "aio_forced_provider".into(),
            forced_provider.map_or(Value::Null, Value::from),
        );
    }
    HistoryDigest::from_value(&properties)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn owner() -> Owner {
        Owner {
            session: "session".into(),
            thread: "thread".into(),
            window: "window".into(),
            context_window: "context".into(),
            turn: "turn".into(),
        }
    }

    fn input(text: &str) -> Value {
        json!({"type":"message","role":"user","content":[{"type":"input_text","text":text}]})
    }

    fn request(
        runtime: &Arc<Runtime>,
        nonce: &str,
        items: &[Value],
        recovered: bool,
    ) -> RequestState {
        RequestState {
            connection: runtime.connection().unwrap(),
            client_ws: true,
            generation: Arc::new(Mutex::new(Generation {
                identity: Some(RecoveryIdentity {
                    owner: owner(),
                    nonce: nonce.into(),
                }),
                expected: HistoryDigest::from_items(items),
                input: HistoryDigest::from_items(items),
                properties: HistoryDigest::default(),
                previous: Some("resp_1".into()),
                committed: false,
                terminal: false,
                incomplete: false,
                failed: false,
                recovered,
                from_trace: None,
                trace_id: "trace_1".into(),
                budget: Budget::default(),
                upstream_ws: false,
            })),
        }
    }

    /// Claim then admit a fresh generation, as ingress does for every create.
    fn begin_fresh(
        runtime: &Arc<Runtime>,
        owner: &Owner,
        nonce: &str,
        items: &[Value],
    ) -> Result<(), &'static str> {
        assert!(runtime
            .claim(owner, Some(nonce), items, &HistoryDigest::default())?
            .is_none());
        let request = request(runtime, nonce, items, false);
        request
            .generation
            .lock_or_recover()
            .identity
            .as_mut()
            .unwrap()
            .owner = owner.clone();
        runtime.begin_generation(&request)?;
        runtime.finish_generation(&request, false);
        Ok(())
    }

    #[test]
    fn recovery_requires_nonce_exact_history_and_single_claim() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("synthetic")];
        let request = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&request).unwrap();
        runtime.suspend(&request).unwrap();
        assert!(runtime
            .claim(&owner(), None, &items, &HistoryDigest::default())
            .is_err());
        assert!(runtime
            .claim(&owner(), Some("wrong"), &items, &HistoryDigest::default())
            .is_err());
        assert!(runtime
            .claim(&owner(), Some(&nonce), &[], &HistoryDigest::default())
            .is_err());
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .unwrap()
            .is_some());
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .is_err());
    }

    #[test]
    fn completed_generation_can_start_a_compacted_context_without_replaying_old_windows() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("first"), input("tool result")];
        let original = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&original).unwrap();
        runtime.finish_generation(&original, true);
        assert_eq!(runtime.issue_nonce(&owner()).unwrap(), nonce);
        let summary = vec![input("compacted history")];
        let mut compacted_owner = owner();
        compacted_owner.context_window = "compacted".into();
        assert!(runtime
            .claim(
                &compacted_owner,
                Some(&nonce),
                &summary,
                &HistoryDigest::default()
            )
            .unwrap()
            .is_none());
        let mut competing_owner = compacted_owner.clone();
        competing_owner.context_window = "competing-compaction".into();
        assert!(runtime
            .claim(
                &competing_owner,
                Some(&nonce),
                &summary,
                &HistoryDigest::default()
            )
            .is_err());
        let compacted = request(&runtime, &nonce, &summary, false);
        compacted
            .generation
            .lock_or_recover()
            .identity
            .as_mut()
            .unwrap()
            .owner = compacted_owner.clone();
        runtime.begin_generation(&compacted).unwrap();
        runtime.finish_generation(&compacted, true);
        assert_eq!(runtime.issue_nonce(&compacted_owner).unwrap(), nonce);
        drop((original, compacted));
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .is_err());
        begin_fresh(&runtime, &compacted_owner, &nonce, &summary).unwrap();
        let extended = vec![input("compacted history"), input("next tool result")];
        assert!(runtime
            .claim(
                &compacted_owner,
                Some(&nonce),
                &extended,
                &HistoryDigest::default()
            )
            .unwrap()
            .is_none());
        let mut next_owner = compacted_owner.clone();
        next_owner.context_window = "compacted-again".into();
        assert!(runtime
            .claim(
                &next_owner,
                Some(&nonce),
                &summary,
                &HistoryDigest::default()
            )
            .unwrap()
            .is_none());
        assert!(runtime
            .claim(
                &compacted_owner,
                Some(&nonce),
                &summary,
                &HistoryDigest::default()
            )
            .is_err());
        assert_eq!(runtime.owners.lock_or_recover().len(), 1);
    }

    #[test]
    fn compaction_cannot_reset_unfinished_failed_or_expired_generation_ownership() {
        for state in [
            "unstarted",
            "active",
            "pending",
            "cancelled",
            "unavailable",
            "expired",
            "disabled",
            "http_only",
        ] {
            let runtime = Arc::new(Runtime::new(true));
            let nonce = runtime.issue_nonce(&owner()).unwrap();
            let items = vec![input("first")];
            let original = request(&runtime, &nonce, &items, false);
            if state != "unstarted" {
                runtime.begin_generation(&original).unwrap();
            }
            match state {
                "pending" => runtime.suspend(&original).unwrap(),
                "cancelled" => runtime.finish_generation(&original, false),
                "unavailable" => {
                    runtime.remember_failure(
                        &original,
                        &GatewayFailure {
                            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            trace_id: "failed".into(),
                            error_code: "GW_ALL_PROVIDERS_UNAVAILABLE".into(),
                            message: "No available providers".into(),
                            attempts: vec![],
                            retry_after_seconds: Some(60),
                        },
                    );
                    runtime.finish_generation(&original, false);
                }
                "expired" | "disabled" | "http_only" => {
                    runtime.finish_generation(&original, true);
                    match state {
                        "expired" => {
                            runtime
                                .owners
                                .lock_or_recover()
                                .get_mut(&owner())
                                .unwrap()
                                .expires = Instant::now()
                        }
                        "disabled" => runtime.set_enabled(false),
                        "http_only" => {
                            runtime
                                .owners
                                .lock_or_recover()
                                .get_mut(&owner())
                                .unwrap()
                                .http_only = true
                        }
                        _ => unreachable!(),
                    }
                }
                _ => {}
            }
            let mut compacted_owner = owner();
            compacted_owner.context_window = "compacted".into();
            let claimed = runtime.claim(
                &compacted_owner,
                Some(&nonce),
                &[input("summary")],
                &HistoryDigest::default(),
            );
            if state == "expired" {
                // Nothing tracks the turn any more: it continues fresh, without old budget.
                assert!(claimed.unwrap().is_none());
            } else {
                assert!(
                    claimed.is_err(),
                    "must preserve {state} generation ownership"
                );
            }
        }
    }

    #[test]
    fn compaction_window_numbers_advance_only_within_the_same_thread() {
        let runtime = Arc::new(Runtime::new(true));
        let mut original_owner = owner();
        original_owner.window = "thread:2".into();
        let nonce = runtime.issue_nonce(&original_owner).unwrap();
        let items = vec![input("first")];
        let original = request(&runtime, &nonce, &items, false);
        original
            .generation
            .lock_or_recover()
            .identity
            .as_mut()
            .unwrap()
            .owner = original_owner.clone();
        runtime.begin_generation(&original).unwrap();
        runtime.finish_generation(&original, true);
        let mut next_owner = original_owner;
        next_owner.context_window = "compacted".into();
        for window in [
            "thread:1",
            "other:3",
            "thread:invalid",
            "other-window",
            "thread:18446744073709551616",
        ] {
            next_owner.window = window.into();
            assert!(runtime
                .claim(&next_owner, Some(&nonce), &items, &HistoryDigest::default())
                .is_err());
        }
        next_owner.window = "thread:3".into();
        assert!(runtime
            .claim(&next_owner, Some(&nonce), &items, &HistoryDigest::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn compaction_requires_the_same_turn_nonce_and_bounds_window_replay_metadata() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("first")];
        let original = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&original).unwrap();
        runtime.finish_generation(&original, true);
        let mut compacted_owner = owner();
        compacted_owner.context_window = "compacted".into();
        for field in ["session", "thread", "window", "turn"] {
            let mut other = compacted_owner.clone();
            match field {
                "session" => other.session = "other".into(),
                "thread" => other.thread = "other".into(),
                "window" => other.window = "other".into(),
                "turn" => other.turn = "other".into(),
                _ => unreachable!(),
            }
            assert!(runtime
                .claim(&other, Some(&nonce), &items, &HistoryDigest::default())
                .is_err());
        }
        assert!(runtime
            .claim(
                &compacted_owner,
                Some("aio-ws-other"),
                &items,
                &HistoryDigest::default()
            )
            .is_err());
        runtime
            .owners
            .lock_or_recover()
            .get_mut(&owner())
            .unwrap()
            .previous_context_windows = (0..MAX_PREVIOUS_CONTEXT_WINDOWS)
            .map(|i| format!("old-{i}"))
            .collect();
        assert!(matches!(
            runtime.claim(
                &compacted_owner,
                Some(&nonce),
                &items,
                &HistoryDigest::default()
            ),
            Err("Responses context window capacity reached")
        ));
        assert!(runtime.owners.lock_or_recover().contains_key(&owner()));
    }

    #[test]
    fn known_unavailability_matches_delta_or_full_history_and_never_claims_a_budget() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("first"), input("tool result")];
        let original = request(&runtime, &nonce, &items, false);
        let body = json!({"model":"model-a","input":items});
        {
            let mut generation = original.generation.lock_or_recover();
            generation.input = HistoryDigest::from_items(&items[1..]);
            generation.properties = request_properties(&body, Some(3));
            generation.budget.http_only = true;
        }
        runtime.begin_generation(&original).unwrap();
        let failure = GatewayFailure {
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
            trace_id: "original-failure".into(),
            error_code: "GW_ALL_PROVIDERS_UNAVAILABLE".into(),
            message: "No available providers: all candidate providers have open circuit breakers"
                .into(),
            attempts: vec![json!({"upstream_sent":false})],
            retry_after_seconds: Some(60),
        };
        runtime.remember_failure(&original, &failure);
        runtime.finish_generation(&original, false);
        let properties = request_properties(&body, Some(3));
        let expires = runtime.owners.lock_or_recover()[&owner()].expires;
        let read = || {
            runtime.known_failure(
                &owner(),
                Some(&nonce),
                &items[1..],
                Some("resp_1"),
                &properties,
            )
        };
        for _ in 0..2 {
            let result = read().unwrap().unwrap();
            assert_eq!(result.trace_id, failure.trace_id);
            assert!(result.attempts.is_empty());
            assert!(result.retry_after_seconds.unwrap() <= 60);
            assert_eq!(runtime.owners.lock_or_recover()[&owner()].expires, expires);
        }
        // Other requests in the turn are not retries of the failure and route normally.
        assert!(runtime
            .known_failure(
                &owner(),
                Some(&nonce),
                &items[1..],
                Some("changed"),
                &properties
            )
            .unwrap()
            .is_none());
        assert!(runtime
            .known_failure(
                &owner(),
                Some(&nonce),
                &items,
                None,
                &request_properties(&body, None)
            )
            .unwrap()
            .is_none());
        assert!(runtime
            .known_failure(&owner(), Some("aio-ws-forged"), &items, None, &properties)
            .is_err());
        assert!(runtime
            .claim_for_transport(&owner(), Some(&nonce), &items, &properties, false)
            .unwrap()
            .is_none());

        // All buffer reservations are occupied; result reading still works over HTTP.
        let _other_connections: Vec<_> = (1..MAX_CONNECTIONS)
            .map(|_| runtime.connection().unwrap())
            .collect();
        assert_eq!(runtime.raw_buffers.available_permits(), 0);
        let headers = HeaderMap::from_iter([
            (axum::http::HeaderName::from_static(TURN_STATE_HEADER), nonce.parse().unwrap()),
            (axum::http::HeaderName::from_static("x-codex-turn-metadata"), json!({"session_id":"session","thread_id":"thread","window_id":"window","context_window_id":"context","turn_id":"turn"}).to_string().parse().unwrap()),
        ]);
        assert!(matches!(
            runtime.prepare_http_recovery(&headers, &body, Some(3)),
            Ok(PreparedRequest::Failure(_))
        ));
        {
            let mut records = runtime.owners.lock_or_recover();
            let known = records.get_mut(&owner()).unwrap().failure.as_mut().unwrap();
            known.retry_at = Instant::now().checked_sub(Duration::from_secs(31)).unwrap();
        }
        // Once providers may have recovered, the same retry is routed again.
        assert!(read().unwrap().is_none());
        assert!(runtime.owners.lock_or_recover()[&owner()].failure.is_none());
        runtime
            .owners
            .lock_or_recover()
            .get_mut(&owner())
            .unwrap()
            .expires = Instant::now();
        assert!(read().unwrap().is_none());
        assert!(runtime
            .prepare_http_recovery(&headers, &body, Some(3))
            .is_err());
    }

    #[test]
    fn unavailability_without_recovery_time_or_after_reset_is_routed_again() {
        for case in ["no_retry_time", "reset"] {
            let runtime = Arc::new(Runtime::new(true));
            let nonce = runtime.issue_nonce(&owner()).unwrap();
            let items = vec![input("first")];
            let original = request(&runtime, &nonce, &items, false);
            runtime.begin_generation(&original).unwrap();
            runtime.remember_failure(
                &original,
                &GatewayFailure {
                    status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    trace_id: "original".into(),
                    error_code: "GW_ALL_PROVIDERS_UNAVAILABLE".into(),
                    message: "No available providers".into(),
                    attempts: vec![],
                    retry_after_seconds: (case == "reset").then_some(60),
                },
            );
            runtime.finish_generation(&original, false);
            drop(original);
            let read = || {
                runtime.known_failure(
                    &owner(),
                    Some(&nonce),
                    &items,
                    None,
                    &HistoryDigest::default(),
                )
            };
            if case == "reset" {
                assert_eq!(read().unwrap().unwrap().trace_id, "original");
                runtime.clear_failures();
            }
            assert!(read().unwrap().is_none());
            assert!(begin_fresh(&runtime, &owner(), &nonce, &items).is_ok());
        }
    }

    #[test]
    fn recovery_signal_and_non_unavailable_failures_are_not_retained_as_results() {
        for (status, code) in [
            (400, "previous_response_not_found"),
            (502, "GW_STREAM_ERROR"),
            (503, "GW_INTERNAL_ERROR"),
        ] {
            let runtime = Arc::new(Runtime::new(true));
            let nonce = runtime.issue_nonce(&owner()).unwrap();
            let items = vec![input("hello")];
            let original = request(&runtime, &nonce, &items, false);
            runtime.begin_generation(&original).unwrap();
            runtime.remember_failure(
                &original,
                &GatewayFailure {
                    status: axum::http::StatusCode::from_u16(status).unwrap(),
                    trace_id: "original".into(),
                    error_code: code.into(),
                    message: "failed".into(),
                    attempts: vec![],
                    retry_after_seconds: None,
                },
            );
            assert!(runtime.owners.lock_or_recover()[&owner()].failure.is_none());
            if code == "previous_response_not_found" {
                runtime.suspend(&original).unwrap();
                runtime.finish_generation(&original, false);
                assert!(runtime
                    .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
                    .unwrap()
                    .is_some());
            } else {
                runtime.finish_generation(&original, false);
                // Codex retries the failed stream with the same turn nonce: no grant, fresh generation.
                assert!(runtime
                    .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
                    .unwrap()
                    .is_none());
            }
        }
    }

    #[test]
    fn completed_recovery_releases_next_generation_and_rejects_old_replay() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let first = vec![input("first")];
        let original = request(&runtime, &nonce, &first, false);
        runtime.begin_generation(&original).unwrap();
        runtime.suspend(&original).unwrap();
        runtime
            .claim(&owner(), Some(&nonce), &first, &HistoryDigest::default())
            .unwrap();
        let recovered = request(&runtime, &nonce, &first, true);
        runtime.begin_generation(&recovered).unwrap();
        runtime.finish_generation(&original, false);
        runtime.finish_generation(&recovered, true);
        drop(original);
        drop(recovered);
        let second = vec![input("first"), input("second")];
        let next = request(&runtime, &nonce, &second, false);
        runtime.begin_generation(&next).unwrap();
        runtime.suspend(&next).unwrap();
        assert!(runtime
            .claim(&owner(), Some(&nonce), &first, &HistoryDigest::default())
            .is_err());
        assert!(runtime
            .claim(&owner(), Some(&nonce), &second, &HistoryDigest::default())
            .unwrap()
            .is_some());
    }

    #[test]
    fn expired_nonce_continues_fresh_while_forged_and_disabled_nonces_are_rejected() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("synthetic")];
        let request = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&request).unwrap();
        assert!(runtime.issue_nonce(&owner()).is_err());
        assert!(runtime.begin_generation(&request).is_err());
        runtime.finish_generation(&request, false);
        runtime
            .owners
            .lock_or_recover()
            .get_mut(&owner())
            .unwrap()
            .expires = Instant::now();
        // A gateway restart or idle expiry forgets the turn; it continues without a budget.
        assert!(begin_fresh(&runtime, &owner(), &nonce, &items).is_ok());
        assert!(runtime
            .claim(
                &owner(),
                Some("aio-ws-forged"),
                &items,
                &HistoryDigest::default()
            )
            .is_err());
        runtime.set_enabled(false);
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .is_err());
    }

    #[test]
    fn recovery_preserves_constraints_and_attempt_deadline() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![
            input("synthetic"),
            json!({"type":"compaction","id":"cmp_checkpoint","encrypted_content":"synthetic-checkpoint"}),
            json!({"type":"compaction_trigger"}),
        ];
        let request = request(&runtime, &nonce, &items, false);
        let deadline = Instant::now() + Duration::from_secs(5);
        let properties = request_properties(&json!({"model":"model-a"}), Some(3));
        {
            let mut generation = request.generation.lock_or_recover();
            generation.properties = properties.clone();
            generation.budget.providers = vec![3, 7];
            generation.budget.provider_id = Some(3);
            generation.budget.retry_index = 2;
            generation.budget.tried_ws.insert(3);
            generation.budget.deadline = Some(deadline);
        }
        runtime.begin_generation(&request).unwrap();
        runtime.suspend(&request).unwrap();
        let mut changed_checkpoint = items.clone();
        changed_checkpoint[1]["encrypted_content"] = json!("changed-checkpoint");
        assert!(runtime
            .claim(&owner(), Some(&nonce), &changed_checkpoint, &properties)
            .is_err());
        assert!(runtime
            .claim(
                &owner(),
                Some(&nonce),
                &items,
                &request_properties(&json!({"model":"model-b"}), Some(3))
            )
            .is_err());
        assert!(runtime
            .claim(
                &owner(),
                Some(&nonce),
                &items,
                &request_properties(&json!({"model":"model-a"}), Some(7))
            )
            .is_err());
        let restored = runtime
            .claim(&owner(), Some(&nonce), &items, &properties)
            .unwrap()
            .unwrap();
        assert_eq!(restored.budget.providers, vec![3, 7]);
        assert_eq!(restored.budget.retry_index, 2);
        assert!(restored.budget.tried_ws.contains(&3));
        assert_eq!(restored.budget.deadline, Some(deadline));
    }

    #[test]
    fn ordinary_http_is_untouched_and_http_recovery_stays_http() {
        let runtime = Arc::new(Runtime::new(true));
        let mut headers = HeaderMap::new();
        let items = vec![input("synthetic")];
        let mut body = json!({"model":"model-a","input":items});
        assert!(runtime
            .prepare_http_recovery(&headers, &body, None)
            .unwrap()
            .unwrap_dispatch()
            .is_none());
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let original = request(&runtime, &nonce, &items, false);
        original.generation.lock_or_recover().properties = request_properties(&body, None);
        runtime.begin_generation(&original).unwrap();
        runtime.suspend(&original).unwrap();
        headers.insert(TURN_STATE_HEADER, nonce.parse().unwrap());
        headers.insert("x-codex-turn-metadata", json!({"session_id":"session","thread_id":"thread","window_id":"window","context_window_id":"context","turn_id":"turn"}).to_string().parse().unwrap());
        let recovered = runtime
            .prepare_http_recovery(&headers, &body, None)
            .unwrap()
            .unwrap_dispatch()
            .unwrap();
        assert!(!recovered.client_ws);
        assert!(recovered.generation.lock_or_recover().budget.http_only);
        assert_eq!(
            recovered.generation.lock_or_recover().from_trace.as_deref(),
            Some("trace_1")
        );
        runtime.finish_generation(&recovered, true);
        body["input"]
            .as_array_mut()
            .unwrap()
            .push(input("next tool output"));
        assert!(runtime
            .claim(
                &owner(),
                Some(&nonce),
                body["input"].as_array().unwrap(),
                &request_properties(&body, None)
            )
            .is_err());
        drop(original);
        drop(recovered);
        let next = runtime
            .prepare_http_recovery(&headers, &body, None)
            .unwrap()
            .unwrap_dispatch()
            .unwrap();
        assert!(!next.generation.lock_or_recover().recovered);
        assert!(next.generation.lock_or_recover().budget.http_only);
    }

    #[test]
    fn connection_limit_http_hint_and_shutdown_are_local() {
        let runtime = Arc::new(Runtime::new(true));
        let mut connections: Vec<_> = (0..20).map(|_| runtime.connection().unwrap()).collect();
        assert_eq!(runtime.raw_buffers.available_permits(), 0);
        assert!(matches!(
            runtime.connection(),
            Err("Responses WebSocket connection limit reached")
        ));
        drop(connections.pop());
        assert!(runtime.connection().is_ok());
        runtime.force_http("window-a");
        assert!(runtime.prefers_http("window-a"));
        assert!(!runtime.prefers_http("window-b"));
        runtime
            .force_http_sessions
            .lock_or_recover()
            .insert("window-a".into(), Instant::now());
        assert!(!runtime.prefers_http("window-a"));
        runtime.stop();
        assert!(!runtime.enabled());
        assert!(*runtime.shutdown.borrow());
        assert!(runtime.connection().is_err());
    }
    #[test]
    fn exhausted_attempt_deadline_is_preserved_for_failover_but_expired_grant_is_not_reissued() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("synthetic")];
        let request = request(&runtime, &nonce, &items, false);
        let deadline = Instant::now();
        request.generation.lock_or_recover().budget.deadline = Some(deadline);
        runtime.begin_generation(&request).unwrap();
        runtime.suspend(&request).unwrap();
        let recovered = runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .unwrap()
            .unwrap();
        assert_eq!(recovered.budget.deadline, Some(deadline));
        runtime
            .owners
            .lock_or_recover()
            .get_mut(&owner())
            .unwrap()
            .pending
            .as_mut()
            .unwrap()
            .expires = Instant::now();
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .unwrap()
            .is_none());
        assert!(runtime.issue_nonce(&owner()).is_err());
    }

    #[test]
    fn unsupported_history_is_not_recoverable_and_owner_windows_are_isolated() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![json!({"type":"unknown","value":"a"})];
        let request = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&request).unwrap();
        assert!(runtime.suspend(&request).is_err());
        let mut other = owner();
        other.window = "other-window".into();
        assert!(runtime
            .claim(&other, Some(&nonce), &items, &HistoryDigest::default())
            .is_err());
        assert!(runtime.issue_nonce(&other).is_ok());
    }

    #[test]
    fn late_duplicate_finish_cannot_retire_a_completed_generation() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("first")];
        let original = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&original).unwrap();
        runtime.finish_generation(&original, true);
        runtime.finish_generation(&original, false);
        let next = vec![input("first"), input("next")];
        assert!(runtime
            .claim(&owner(), Some(&nonce), &next, &HistoryDigest::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn ended_generation_releases_the_turn_for_the_next_codex_request() {
        // Codex cancels a stream to merge steering or sub-agent input, then resends
        // the turn on a new socket with the same nonce.
        for dropped in [false, true] {
            let runtime = Arc::new(Runtime::new(true));
            let nonce = runtime.issue_nonce(&owner()).unwrap();
            let first = vec![input("first")];
            let completed = request(&runtime, &nonce, &first, false);
            runtime.begin_generation(&completed).unwrap();
            runtime.finish_generation(&completed, true);
            drop(completed);
            let second = vec![input("first"), input("second")];
            let ended = request(&runtime, &nonce, &second, false);
            runtime.begin_generation(&ended).unwrap();
            if dropped {
                drop(ended);
            } else {
                runtime.finish_generation(&ended, false);
            }
            begin_fresh(&runtime, &owner(), &nonce, &first).unwrap();
            assert!(runtime
                .claim(
                    &owner(),
                    Some("aio-ws-forged"),
                    &second,
                    &HistoryDigest::default()
                )
                .is_err());
            let steered = vec![input("first"), input("second"), input("steer")];
            assert!(runtime
                .claim(&owner(), Some(&nonce), &steered, &HistoryDigest::default())
                .unwrap()
                .is_none());
            let next = request(&runtime, &nonce, &steered, false);
            runtime.begin_generation(&next).unwrap();
            assert!(runtime
                .claim(&owner(), Some(&nonce), &steered, &HistoryDigest::default())
                .is_err());
            runtime.finish_generation(&next, true);
            assert_eq!(
                runtime.owners.lock_or_recover()[&owner()].completed,
                Some(HistoryDigest::from_items(&steered))
            );
        }
    }

    #[test]
    fn generation_of_a_turn_can_be_retried_after_it_fails_or_completes() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("first")];
        let failed = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&failed).unwrap();
        runtime.finish_generation(&failed, false);
        drop(failed);
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .unwrap()
            .is_none());
        let retry = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&retry).unwrap();
        runtime.finish_generation(&retry, true);
        drop(retry);
        // Codex retries a completed stream it never saw finish with the same input.
        begin_fresh(&runtime, &owner(), &nonce, &items).unwrap();
    }

    #[test]
    fn longer_resend_supersedes_a_pending_grant_but_stale_replays_keep_it() {
        for superseded in [false, true] {
            let runtime = Arc::new(Runtime::new(true));
            let nonce = runtime.issue_nonce(&owner()).unwrap();
            let items = vec![input("first"), input("tool result")];
            let original = request(&runtime, &nonce, &items, false);
            runtime.begin_generation(&original).unwrap();
            runtime.suspend(&original).unwrap();
            runtime.finish_generation(&original, false);
            drop(original);
            assert!(runtime
                .claim(
                    &owner(),
                    Some(&nonce),
                    &items[..1],
                    &HistoryDigest::default()
                )
                .is_err());
            if superseded {
                let steered = vec![input("first"), input("tool result"), input("steer")];
                assert!(begin_fresh(&runtime, &owner(), &nonce, &steered).is_ok());
                assert!(runtime.owners.lock_or_recover()[&owner()].pending.is_none());
            } else {
                assert!(runtime
                    .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
                    .unwrap()
                    .is_some());
            }
        }
    }

    #[test]
    fn ended_recovered_generation_does_not_revive_its_grant() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("first")];
        let original = request(&runtime, &nonce, &items, false);
        runtime.begin_generation(&original).unwrap();
        runtime.suspend(&original).unwrap();
        runtime.finish_generation(&original, false);
        drop(original);
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .unwrap()
            .is_some());
        let recovered = request(&runtime, &nonce, &items, true);
        runtime.begin_generation(&recovered).unwrap();
        runtime.finish_generation(&recovered, false);
        drop(recovered);
        assert!(runtime
            .claim(&owner(), Some(&nonce), &items, &HistoryDigest::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn http_recovery_rejects_invalid_present_identity_before_claim() {
        let runtime = Arc::new(Runtime::new(true));
        let nonce = runtime.issue_nonce(&owner()).unwrap();
        let items = vec![input("first")];
        let original = request(&runtime, &nonce, &items, false);
        let body = json!({"model":"model-a","input":items});
        original.generation.lock_or_recover().properties = request_properties(&body, None);
        runtime.begin_generation(&original).unwrap();
        runtime.suspend(&original).unwrap();
        let metadata = json!({"session_id":"session","thread_id":"thread","window_id":"window","context_window_id":"context","turn_id":"turn"}).to_string();
        let mut headers = HeaderMap::new();
        headers.insert(TURN_STATE_HEADER, nonce.parse().unwrap());
        headers.insert("x-codex-turn-metadata", metadata.parse().unwrap());
        for invalid in [
            Value::Null,
            Value::Bool(false),
            Value::String("invalid".into()),
        ] {
            let mut bad_nonce = body.clone();
            bad_nonce["client_metadata"] = json!({TURN_STATE_HEADER:invalid});
            assert!(matches!(
                runtime.prepare_http_recovery(&headers, &bad_nonce, None),
                Err("conflicting Responses owner nonce")
            ));
            let mut bad_owner = body.clone();
            bad_owner["client_metadata"] = json!({"x-codex-turn-metadata":invalid});
            assert!(matches!(
                runtime.prepare_http_recovery(&headers, &bad_owner, None),
                Err("invalid Codex turn metadata")
            ));
        }
        let mut bad_headers = headers.clone();
        bad_headers.insert(
            TURN_STATE_HEADER,
            axum::http::HeaderValue::from_bytes(&[0xff]).unwrap(),
        );
        let mut body_nonce = body.clone();
        body_nonce["client_metadata"] = json!({TURN_STATE_HEADER:nonce});
        assert!(matches!(
            runtime.prepare_http_recovery(&bad_headers, &body_nonce, None),
            Err("conflicting Responses owner nonce")
        ));
        bad_headers = headers.clone();
        bad_headers.insert("x-codex-turn-metadata", "invalid".parse().unwrap());
        body_nonce["client_metadata"]["x-codex-turn-metadata"] = Value::String(metadata);
        assert!(matches!(
            runtime.prepare_http_recovery(&bad_headers, &body_nonce, None),
            Err("invalid Codex turn metadata")
        ));
        let others: Vec<_> = (1..MAX_CONNECTIONS)
            .map(|_| runtime.connection().unwrap())
            .collect();
        assert!(matches!(
            runtime.connection(),
            Err("Responses WebSocket connection limit reached")
        ));
        assert!(matches!(
            runtime.prepare_http_recovery(&headers, &body, None),
            Err("Responses WebSocket raw buffer budget exhausted")
        ));
        // The original socket closes before the CLI's HTTP fallback is admitted.
        drop(original);
        let recovered = runtime
            .prepare_http_recovery(&headers, &body, None)
            .unwrap()
            .unwrap_dispatch()
            .unwrap();
        assert!(recovered.generation.lock_or_recover().recovered);
        assert!(matches!(
            runtime.connection(),
            Err("Responses WebSocket raw buffer budget exhausted")
        ));
        drop(others);
        drop(recovered);
        assert_eq!(runtime.raw_buffers.available_permits(), RAW_BUFFER_BUDGET);
    }

    #[test]
    fn expired_cooldown_allows_one_probe_and_cancel_does_not_claim_health() {
        let runtime = Arc::new(Runtime::new(true));
        assert!(runtime.try_ws_probe("healthy").unwrap().is_none());
        runtime.cool("endpoint".into());
        assert!(runtime.cooling("endpoint"));
        assert!(runtime.try_ws_probe("endpoint").is_err());
        runtime
            .cooldowns
            .lock_or_recover()
            .get_mut("endpoint")
            .unwrap()
            .until = Instant::now();
        let probe = runtime.try_ws_probe("endpoint").unwrap().unwrap();
        assert!(runtime.cooling("endpoint"));
        for _ in 0..10 {
            assert!(runtime.try_ws_probe("endpoint").is_err());
        }
        drop(probe);
        assert!(!runtime.cooling("endpoint"));
        let probe = runtime.try_ws_probe("endpoint").unwrap().unwrap();
        probe.succeeded();
        assert!(runtime.try_ws_probe("endpoint").unwrap().is_none());
    }

    #[test]
    fn failed_probe_starts_new_cooldown_and_stale_success_cannot_clear_it() {
        let runtime = Arc::new(Runtime::new(true));
        runtime.cool("endpoint".into());
        runtime
            .cooldowns
            .lock_or_recover()
            .get_mut("endpoint")
            .unwrap()
            .until = Instant::now();
        let probe = runtime.try_ws_probe("endpoint").unwrap().unwrap();
        runtime.cool("endpoint".into());
        probe.succeeded();
        assert!(runtime.cooling("endpoint"));
        assert!(runtime.try_ws_probe("endpoint").is_err());
    }

    #[test]
    fn raw_buffer_budget_is_shared_and_released_by_last_connection_owner() {
        let runtime = Arc::new(Runtime::new(true));
        let first = runtime.connection().unwrap();
        let others: Vec<_> = (1..MAX_CONNECTIONS)
            .map(|_| runtime.connection().unwrap())
            .collect();
        assert_eq!(runtime.raw_buffers.available_permits(), 0);
        assert!(matches!(
            runtime.reserve_raw_buffers(),
            Err("Responses WebSocket raw buffer budget exhausted")
        ));
        let held = first.clone();
        drop(first);
        assert_eq!(runtime.raw_buffers.available_permits(), 0);
        drop(held);
        assert_eq!(
            runtime.raw_buffers.available_permits(),
            RAW_BUFFER_RESERVATION as usize
        );
        runtime.stop();
        assert_eq!(
            runtime.raw_buffers.available_permits(),
            RAW_BUFFER_RESERVATION as usize
        );
        drop(others);
        assert_eq!(runtime.raw_buffers.available_permits(), RAW_BUFFER_BUDGET);
    }
}
