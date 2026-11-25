//! Usage: Managed Extension Host process instance reuse and disposal.

use super::extension_host::ExtensionHostInstance;
use super::privacy_redaction_service::PrivacyRedactionService;
use crate::app::app_state::{ensure_db_ready, DbInitState};
use crate::app::plugins::runtime_lifecycle::PluginRuntimeInstanceRegistry;
use crate::db;
use crate::domain::plugins::{PluginDetail, PluginManifest, PluginRuntime};
use crate::gateway::plugins::context::{
    GatewayHookAction, GatewayHookResult, GatewayPluginHookName, GatewayVisibleHookContext,
};
use crate::gateway::plugins::permissions::GatewayPluginError;
use crate::shared::error::{AppError, AppResult};
use serde_json::{json, Value};
use sha2::Digest;
use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

const DEFAULT_MAX_WARM_INSTANCES: usize = 8;
const DEFAULT_IDLE_RECYCLE: Duration = Duration::from_secs(120);

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ExtensionHostInstanceKey {
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) installed_dir: String,
    pub(crate) main: String,
    pub(crate) runtime_kind: String,
    pub(crate) runtime_language: String,
    pub(crate) contribution_hash: String,
    pub(crate) call_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExtensionHostRegistryLimits {
    pub(crate) max_warm_instances: usize,
    pub(crate) idle_recycle: Duration,
}

impl Default for ExtensionHostRegistryLimits {
    fn default() -> Self {
        Self {
            max_warm_instances: DEFAULT_MAX_WARM_INSTANCES,
            idle_recycle: DEFAULT_IDLE_RECYCLE,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ExtensionHostCommandOutput {
    pub(crate) value: Value,
    pub(crate) cold_start: bool,
}

trait ExtensionHostProcess: Send {
    fn execute_command<'a>(
        &'a mut self,
        command: &'a str,
        args: Value,
    ) -> BoxFuture<'a, AppResult<Value>>;
    fn execute_gateway_hook<'a>(
        &'a mut self,
        hook: &'a str,
        context: Value,
    ) -> BoxFuture<'a, AppResult<Value>>;
    fn is_running(&mut self) -> bool;
    fn dispose<'a>(&'a mut self) -> BoxFuture<'a, ()>;
}

trait ExtensionHostFactory: Send + Sync {
    fn start<'a>(
        &'a self,
        detail: PluginDetail,
        db: db::Db,
        call_timeout: Duration,
    ) -> BoxFuture<'a, AppResult<Box<dyn ExtensionHostProcess>>>;
}

#[allow(dead_code)]
struct RealExtensionHostFactory {
    privacy_redaction: Arc<PrivacyRedactionService>,
}

impl ExtensionHostFactory for RealExtensionHostFactory {
    fn start<'a>(
        &'a self,
        detail: PluginDetail,
        db: db::Db,
        call_timeout: Duration,
    ) -> BoxFuture<'a, AppResult<Box<dyn ExtensionHostProcess>>> {
        Box::pin(async move {
            let plugin_root = plugin_root(&detail)?;
            let host =
                ExtensionHostInstance::start_with_host_api_and_privacy_redaction_with_timeout(
                    detail.manifest.clone(),
                    plugin_root,
                    db,
                    self.privacy_redaction.clone(),
                    call_timeout,
                )
                .await?;
            Ok(Box::new(RealExtensionHostProcess { host }) as Box<dyn ExtensionHostProcess>)
        })
    }
}

#[allow(dead_code)]
struct RealExtensionHostProcess {
    host: ExtensionHostInstance,
}

impl ExtensionHostProcess for RealExtensionHostProcess {
    fn execute_command<'a>(
        &'a mut self,
        command: &'a str,
        args: Value,
    ) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move { self.host.execute_command(command, args).await })
    }

    fn execute_gateway_hook<'a>(
        &'a mut self,
        hook: &'a str,
        context: Value,
    ) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move { self.host.execute_gateway_hook(hook, context).await })
    }

    fn is_running(&mut self) -> bool {
        self.host.is_running()
    }

    fn dispose<'a>(&'a mut self) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.host.dispose().await;
        })
    }
}

struct ManagedExtensionHostInstance {
    process: Mutex<Box<dyn ExtensionHostProcess>>,
    last_used: StdMutex<Instant>,
}

impl ManagedExtensionHostInstance {
    fn new(process: Box<dyn ExtensionHostProcess>, last_used: Instant) -> Self {
        Self {
            process: Mutex::new(process),
            last_used: StdMutex::new(last_used),
        }
    }

    async fn execute_if_running(
        &self,
        command: &str,
        args: Value,
        now: Instant,
    ) -> AppResult<Option<Value>> {
        let mut process = self.process.lock().await;
        if !process.is_running() {
            return Ok(None);
        }
        let value = process.execute_command(command, args).await?;
        *self
            .last_used
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = now;
        Ok(Some(value))
    }

    async fn execute_gateway_hook_if_running(
        &self,
        hook: &str,
        context: Value,
        now: Instant,
    ) -> AppResult<Option<Value>> {
        let mut process = self.process.lock().await;
        if !process.is_running() {
            return Ok(None);
        }
        let value = process.execute_gateway_hook(hook, context).await?;
        *self
            .last_used
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = now;
        Ok(Some(value))
    }

    async fn dispose(&self) {
        self.process.lock().await.dispose().await;
    }

    fn last_used(&self) -> Instant {
        *self
            .last_used
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub(crate) struct ExtensionHostInstanceRegistry {
    db: db::Db,
    operation_gate: RwLock<()>,
    instances: Mutex<BTreeMap<ExtensionHostInstanceKey, Arc<ManagedExtensionHostInstance>>>,
    plugin_locks: Mutex<BTreeMap<String, Arc<Mutex<()>>>>,
    limits: ExtensionHostRegistryLimits,
    factory: Arc<dyn ExtensionHostFactory>,
}

impl ExtensionHostInstanceRegistry {
    #[allow(dead_code)]
    pub(crate) fn new(db: db::Db) -> Self {
        Self::new_with_privacy_redaction(db, Arc::new(PrivacyRedactionService::default()))
    }

    pub(crate) fn new_with_privacy_redaction(
        db: db::Db,
        privacy_redaction: Arc<PrivacyRedactionService>,
    ) -> Self {
        Self::with_factory(
            db,
            Arc::new(RealExtensionHostFactory { privacy_redaction }),
            ExtensionHostRegistryLimits::default(),
        )
    }

    fn with_factory(
        db: db::Db,
        factory: Arc<dyn ExtensionHostFactory>,
        limits: ExtensionHostRegistryLimits,
    ) -> Self {
        Self {
            db,
            operation_gate: RwLock::new(()),
            instances: Mutex::new(BTreeMap::new()),
            plugin_locks: Mutex::new(BTreeMap::new()),
            limits,
            factory,
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn execute_command(
        &self,
        detail: PluginDetail,
        command: &str,
        args: Value,
    ) -> AppResult<ExtensionHostCommandOutput> {
        self.execute_command_with_now(detail, command, args, Instant::now())
            .await
    }

    async fn execute_command_with_now(
        &self,
        detail: PluginDetail,
        command: &str,
        args: Value,
        now: Instant,
    ) -> AppResult<ExtensionHostCommandOutput> {
        let _operation_guard = self.operation_gate.read().await;
        let key = ExtensionHostInstanceKey::from_plugin_detail(&detail)?;
        let plugin_lock = self.plugin_lock_for(&key.plugin_id).await;
        let _plugin_guard = plugin_lock.lock().await;

        if let Some(value) = self
            .execute_warm_instance(&key, command, args.clone(), now)
            .await?
        {
            return Ok(ExtensionHostCommandOutput {
                value,
                cold_start: false,
            });
        }

        let mut disposals = {
            let mut instances = self.instances.lock().await;
            let mut disposals = remove_same_plugin_with_different_key(&mut instances, &key);
            disposals.extend(remove_idle_locked(
                &mut instances,
                self.limits.idle_recycle,
                now,
            ));
            disposals
        };
        dispose_instances(disposals.drain(..)).await;

        let mut process = self
            .factory
            .start(
                detail,
                self.db.clone(),
                crate::app::plugins::extension_host::DEFAULT_EXTENSION_HOST_CALL_TIMEOUT,
            )
            .await?;
        let value = match process.execute_command(command, args).await {
            Ok(value) => value,
            Err(error) => {
                process.dispose().await;
                return Err(error);
            }
        };
        let instance = Arc::new(ManagedExtensionHostInstance::new(process, now));
        let disposals = {
            let mut instances = self.instances.lock().await;
            instances.insert(key, instance);
            remove_lru_over_limit_locked(&mut instances, self.limits.max_warm_instances)
        };
        dispose_instances(disposals).await;

        Ok(ExtensionHostCommandOutput {
            value,
            cold_start: true,
        })
    }

    pub(crate) async fn execute_gateway_hook(
        &self,
        detail: PluginDetail,
        hook: &str,
        context: GatewayVisibleHookContext,
        call_timeout: Duration,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        self.execute_gateway_hook_with_now(detail, hook, context, call_timeout, Instant::now())
            .await
    }

    async fn execute_gateway_hook_with_now(
        &self,
        detail: PluginDetail,
        hook: &str,
        context: GatewayVisibleHookContext,
        call_timeout: Duration,
        now: Instant,
    ) -> Result<GatewayHookResult, GatewayPluginError> {
        let context_value = serde_json::to_value(&context).map_err(|err| {
            GatewayPluginError::new(
                "PLUGIN_EXTENSION_HOST_INVALID_CONTEXT",
                format!("failed to encode extension host gateway context: {err}"),
            )
        })?;
        let payload = json!({
            "hook": hook,
            "traceId": context.trace_id.clone(),
            "config": detail.config.clone(),
            "context": context_value,
        });
        let _operation_guard = self.operation_gate.read().await;
        let key = ExtensionHostInstanceKey::from_gateway_plugin_detail(&detail, call_timeout)
            .map_err(extension_host_gateway_error)?;
        let plugin_lock = self.plugin_lock_for(&key.plugin_id).await;
        let _plugin_guard = plugin_lock.lock().await;

        if let Some(value) = self
            .execute_gateway_hook_warm_instance(&key, hook, payload.clone(), now)
            .await
            .map_err(extension_host_gateway_error)?
        {
            return gateway_hook_result_from_extension_host_output(hook, &context, value);
        }

        let mut disposals = {
            let mut instances = self.instances.lock().await;
            let mut disposals = remove_same_plugin_with_different_key(&mut instances, &key);
            disposals.extend(remove_idle_locked(
                &mut instances,
                self.limits.idle_recycle,
                now,
            ));
            disposals
        };
        dispose_instances(disposals.drain(..)).await;

        let mut process = self
            .factory
            .start(detail, self.db.clone(), call_timeout)
            .await
            .map_err(extension_host_gateway_error)?;
        let value = match process.execute_gateway_hook(hook, payload).await {
            Ok(value) => value,
            Err(error) => {
                process.dispose().await;
                return Err(extension_host_gateway_error(error));
            }
        };
        let result = gateway_hook_result_from_extension_host_output(hook, &context, value)?;
        let instance = Arc::new(ManagedExtensionHostInstance::new(process, now));
        let disposals = {
            let mut instances = self.instances.lock().await;
            instances.insert(key, instance);
            remove_lru_over_limit_locked(&mut instances, self.limits.max_warm_instances)
        };
        dispose_instances(disposals).await;

        Ok(result)
    }

    #[allow(dead_code)]
    pub(crate) async fn dispose_plugin(&self, plugin_id: &str) {
        let _operation_guard = self.operation_gate.read().await;
        let plugin_lock = self.plugin_lock_for(plugin_id).await;
        let plugin_guard = plugin_lock.lock().await;
        let disposals = {
            let mut instances = self.instances.lock().await;
            let keys = instances
                .keys()
                .filter(|key| key.plugin_id == plugin_id)
                .cloned()
                .collect::<Vec<_>>();
            keys.into_iter()
                .filter_map(|key| instances.remove(&key))
                .collect::<Vec<_>>()
        };
        dispose_instances(disposals).await;
        drop(plugin_guard);
        self.remove_plugin_lock_if_unused(plugin_id, &plugin_lock)
            .await;
    }

    pub(crate) async fn retain_plugins(&self, active_plugin_ids: &HashSet<String>) {
        let _operation_guard = self.operation_gate.write().await;
        let removals = {
            let mut instances = self.instances.lock().await;
            let keys = instances
                .keys()
                .filter(|key| !active_plugin_ids.contains(&key.plugin_id))
                .cloned()
                .collect::<Vec<_>>();
            keys.into_iter()
                .filter_map(|key| {
                    instances
                        .remove(&key)
                        .map(|instance| (key.plugin_id, instance))
                })
                .collect::<Vec<_>>()
        };
        let removed_plugin_ids = removals
            .iter()
            .map(|(plugin_id, _)| plugin_id.clone())
            .collect::<HashSet<_>>();
        dispose_instances(removals.into_iter().map(|(_, instance)| instance)).await;
        for plugin_id in removed_plugin_ids {
            self.remove_plugin_lock_if_orphaned(&plugin_id).await;
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn dispose_idle(&self, now: Instant) {
        let _operation_guard = self.operation_gate.read().await;
        let disposals = {
            let mut instances = self.instances.lock().await;
            remove_idle_locked(&mut instances, self.limits.idle_recycle, now)
        };
        dispose_instances(disposals).await;
    }

    pub(crate) async fn dispose_all(&self) {
        let _operation_guard = self.operation_gate.write().await;
        let instances = {
            let mut instances = self.instances.lock().await;
            std::mem::take(&mut *instances)
                .into_values()
                .collect::<Vec<_>>()
        };
        dispose_instances(instances).await;
        self.plugin_locks.lock().await.clear();
    }

    #[cfg(test)]
    fn new_for_tests(
        factory: Arc<dyn ExtensionHostFactory>,
        limits: ExtensionHostRegistryLimits,
    ) -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&temp.path().join("registry.db")).expect("init db");
        Self::with_factory(db, factory, limits)
    }

    #[cfg(test)]
    pub(crate) fn new_real_for_tests() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&temp.path().join("registry.db")).expect("init db");
        Self::new(db)
    }

    #[cfg(test)]
    async fn instance_count(&self) -> usize {
        self.instances.lock().await.len()
    }

    #[cfg(test)]
    pub(crate) async fn instance_count_for_tests(&self) -> usize {
        self.instance_count().await
    }

    #[cfg(test)]
    async fn plugin_instance_count(&self, plugin_id: &str) -> usize {
        self.instances
            .lock()
            .await
            .keys()
            .filter(|key| key.plugin_id == plugin_id)
            .count()
    }

    async fn execute_warm_instance(
        &self,
        key: &ExtensionHostInstanceKey,
        command: &str,
        args: Value,
        now: Instant,
    ) -> AppResult<Option<Value>> {
        let instance = { self.instances.lock().await.get(key).cloned() };
        let Some(instance) = instance else {
            return Ok(None);
        };

        match instance.execute_if_running(command, args, now).await {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => {
                self.remove_warm_instance_if_current(key, &instance).await;
                Ok(None)
            }
            Err(error) => {
                self.remove_warm_instance_if_current(key, &instance).await;
                Err(error)
            }
        }
    }

    async fn execute_gateway_hook_warm_instance(
        &self,
        key: &ExtensionHostInstanceKey,
        hook: &str,
        context: Value,
        now: Instant,
    ) -> AppResult<Option<Value>> {
        let instance = { self.instances.lock().await.get(key).cloned() };
        let Some(instance) = instance else {
            return Ok(None);
        };

        match instance
            .execute_gateway_hook_if_running(hook, context, now)
            .await
        {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => {
                self.remove_warm_instance_if_current(key, &instance).await;
                Ok(None)
            }
            Err(error) => {
                self.remove_warm_instance_if_current(key, &instance).await;
                Err(error)
            }
        }
    }

    async fn remove_warm_instance_if_current(
        &self,
        key: &ExtensionHostInstanceKey,
        instance: &Arc<ManagedExtensionHostInstance>,
    ) {
        let removed = {
            let mut instances = self.instances.lock().await;
            let should_remove = instances
                .get(key)
                .filter(|current| Arc::ptr_eq(current, instance))
                .is_some();
            should_remove.then(|| instances.remove(key)).flatten()
        };
        if let Some(instance) = removed {
            instance.dispose().await;
        }
    }

    async fn plugin_lock_for(&self, plugin_id: &str) -> Arc<Mutex<()>> {
        let mut plugin_locks = self.plugin_locks.lock().await;
        plugin_locks
            .entry(plugin_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    async fn remove_plugin_lock_if_unused(&self, plugin_id: &str, plugin_lock: &Arc<Mutex<()>>) {
        let mut plugin_locks = self.plugin_locks.lock().await;
        let should_remove = plugin_locks.get(plugin_id).is_some_and(|current| {
            Arc::ptr_eq(current, plugin_lock) && Arc::strong_count(current) == 2
        });
        if should_remove {
            plugin_locks.remove(plugin_id);
        }
    }

    async fn remove_plugin_lock_if_orphaned(&self, plugin_id: &str) {
        let mut plugin_locks = self.plugin_locks.lock().await;
        let should_remove = plugin_locks
            .get(plugin_id)
            .is_some_and(|current| Arc::strong_count(current) == 1);
        if should_remove {
            plugin_locks.remove(plugin_id);
        }
    }
}

pub(crate) struct ExtensionHostInstanceLifecycleRegistry {
    registry: Arc<ExtensionHostInstanceRegistry>,
}

impl ExtensionHostInstanceLifecycleRegistry {
    pub(crate) fn new(registry: Arc<ExtensionHostInstanceRegistry>) -> Self {
        Self { registry }
    }
}

impl PluginRuntimeInstanceRegistry for ExtensionHostInstanceLifecycleRegistry {
    fn retain_for_plugins(&self, plugins: &[PluginDetail]) {
        let active_plugin_ids = plugins
            .iter()
            .map(|plugin| plugin.summary.plugin_id.clone())
            .collect::<HashSet<_>>();
        let registry = self.registry.clone();
        tauri::async_runtime::spawn(async move {
            registry.retain_plugins(&active_plugin_ids).await;
        });
    }

    fn dispose_plugin(&self, plugin_id: &str) {
        let registry = self.registry.clone();
        let plugin_id = plugin_id.to_string();
        tauri::async_runtime::spawn(async move {
            registry.dispose_plugin(&plugin_id).await;
        });
    }

    fn dispose_all(&self) {
        let registry = self.registry.clone();
        tauri::async_runtime::spawn(async move {
            registry.dispose_all().await;
        });
    }
}

impl ExtensionHostInstanceKey {
    pub(crate) fn from_plugin_detail(detail: &PluginDetail) -> AppResult<Self> {
        let (runtime_kind, runtime_language) = match &detail.manifest.runtime {
            PluginRuntime::ExtensionHost { language } => {
                ("extensionHost".to_string(), language.clone())
            }
        };
        let main = detail
            .manifest
            .main
            .as_ref()
            .filter(|main| !main.trim().is_empty())
            .cloned()
            .ok_or_else(|| {
                AppError::new("PLUGIN_MISSING_MAIN", "extensionHost runtime requires main")
            })?;
        Ok(Self {
            plugin_id: detail.manifest.id.clone(),
            version: detail.manifest.version.clone(),
            installed_dir: plugin_root(detail)?.display().to_string(),
            main,
            runtime_kind,
            runtime_language,
            contribution_hash: contribution_hash(&detail.manifest),
            call_timeout_ms: None,
        })
    }

    pub(crate) fn from_gateway_plugin_detail(
        detail: &PluginDetail,
        call_timeout: Duration,
    ) -> AppResult<Self> {
        let mut key = Self::from_plugin_detail(detail)?;
        key.call_timeout_ms = Some(call_timeout_millis(call_timeout));
        Ok(key)
    }
}

fn call_timeout_millis(call_timeout: Duration) -> u64 {
    call_timeout.as_millis().try_into().unwrap_or(u64::MAX)
}

#[derive(Default)]
pub(crate) struct ExtensionHostRuntimeState {
    registry: Mutex<Option<Arc<ExtensionHostInstanceRegistry>>>,
}

impl ExtensionHostRuntimeState {
    #[allow(dead_code)]
    pub(crate) async fn registry<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        db_state: &DbInitState,
    ) -> AppResult<Arc<ExtensionHostInstanceRegistry>> {
        // Release the lock before the slow initialization below.
        let registry = self.registry.lock().await.clone();
        if let Some(registry) = registry {
            return Ok(registry);
        }

        let db = ensure_db_ready(app, db_state).await?;
        let mut guard = self.registry.lock().await;
        if let Some(registry) = guard.as_ref() {
            return Ok(registry.clone());
        }
        let registry = Arc::new(ExtensionHostInstanceRegistry::new(db));
        *guard = Some(registry.clone());
        Ok(registry)
    }

    pub(crate) async fn dispose_all(&self) {
        let registry = { self.registry.lock().await.clone() };
        if let Some(registry) = registry {
            registry.dispose_all().await;
        }
    }

    pub(crate) async fn dispose_plugin_if_initialized(&self, plugin_id: &str) {
        let registry = { self.registry.lock().await.clone() };
        if let Some(registry) = registry {
            registry.dispose_plugin(plugin_id).await;
        }
    }

    #[cfg(test)]
    async fn set_registry_for_tests(&self, registry: Arc<ExtensionHostInstanceRegistry>) {
        *self.registry.lock().await = Some(registry);
    }
}

fn remove_same_plugin_with_different_key(
    instances: &mut BTreeMap<ExtensionHostInstanceKey, Arc<ManagedExtensionHostInstance>>,
    key: &ExtensionHostInstanceKey,
) -> Vec<Arc<ManagedExtensionHostInstance>> {
    let keys = instances
        .keys()
        .filter(|existing| existing.plugin_id == key.plugin_id && *existing != key)
        .cloned()
        .collect::<Vec<_>>();
    keys.into_iter()
        .filter_map(|key| instances.remove(&key))
        .collect()
}

fn remove_idle_locked(
    instances: &mut BTreeMap<ExtensionHostInstanceKey, Arc<ManagedExtensionHostInstance>>,
    idle_recycle: Duration,
    now: Instant,
) -> Vec<Arc<ManagedExtensionHostInstance>> {
    let idle_keys = instances
        .iter()
        .filter(|(_, instance)| {
            now.checked_duration_since(instance.last_used())
                .is_some_and(|elapsed| elapsed >= idle_recycle)
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    idle_keys
        .into_iter()
        .filter_map(|key| instances.remove(&key))
        .collect()
}

fn remove_lru_over_limit_locked(
    instances: &mut BTreeMap<ExtensionHostInstanceKey, Arc<ManagedExtensionHostInstance>>,
    max_warm_instances: usize,
) -> Vec<Arc<ManagedExtensionHostInstance>> {
    let mut disposals = Vec::new();
    while instances.len() > max_warm_instances {
        let Some(key) = instances
            .iter()
            .min_by_key(|(_, instance)| instance.last_used())
            .map(|(key, _)| key.clone())
        else {
            return disposals;
        };
        if let Some(instance) = instances.remove(&key) {
            disposals.push(instance);
        }
    }
    disposals
}

async fn dispose_instances(instances: impl IntoIterator<Item = Arc<ManagedExtensionHostInstance>>) {
    for instance in instances {
        instance.dispose().await;
    }
}

fn plugin_root(detail: &PluginDetail) -> AppResult<PathBuf> {
    detail
        .installed_dir
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| {
            AppError::new(
                "PLUGIN_EXTENSION_HOST_ROOT_UNAVAILABLE",
                format!(
                    "plugin {} does not have an installed extension host directory",
                    detail.summary.plugin_id
                ),
            )
        })
}

fn contribution_hash(manifest: &PluginManifest) -> String {
    let bytes = serde_json::to_vec(&json!({
        "runtime": manifest.runtime,
        "main": manifest.main,
        "activationEvents": manifest.activation_events,
        "contributes": manifest.contributes,
        "capabilities": manifest.capabilities,
        "permissions": manifest.permissions,
    }))
    .unwrap_or_default();
    format!("{:x}", sha2::Sha256::digest(bytes))
}

fn extension_host_gateway_error(err: AppError) -> GatewayPluginError {
    match err.code() {
        "PLUGIN_EXTENSION_HOST_FORBIDDEN" => GatewayPluginError::new(
            "PLUGIN_PERMISSION_DENIED",
            format!("extension host gateway hook permission denied: {err}"),
        ),
        "PLUGIN_EXTENSION_CALL_TIMEOUT" => GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_TIMEOUT",
            format!("extension host gateway hook timed out: {err}"),
        ),
        "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT" | "PLUGIN_EXTENSION_HOST_DECODE_FAILED" => {
            GatewayPluginError::new(
                "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
                format!("extension host gateway hook returned invalid output: {err}"),
            )
        }
        _ => GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_GATEWAY_FAILED",
            format!("extension host gateway hook failed: {err}"),
        ),
    }
}

fn gateway_hook_result_from_extension_host_output(
    hook: &str,
    context: &GatewayVisibleHookContext,
    value: Value,
) -> Result<GatewayHookResult, GatewayPluginError> {
    let object = value.as_object().ok_or_else(|| {
        GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            "extension host gateway hook output must be a JSON object",
        )
    })?;
    if object.contains_key("contextPatch") {
        return Err(GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            "extension host gateway hook output used legacy contextPatch; use requestBody, responseBody, streamChunk, logMessage, or headers",
        ));
    }
    let action = object
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            GatewayPluginError::new(
                "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
                "extension host gateway hook output must include string action",
            )
        })?;
    let mut result = GatewayHookResult::continue_unchanged();
    match action {
        "continue" | "pass" => {}
        "warn" => {
            result.reason = Some(required_string(object, "message", "warn action")?);
        }
        "block" => {
            if hook_kind(hook) == Some(crate::gateway::plugins::contract::HookKind::Log) {
                return Err(GatewayPluginError::new(
                    "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
                    format!("block action is not allowed in {hook}"),
                ));
            }
            result.action = GatewayHookAction::Block;
            result.reason = optional_string(object, "reason")?;
        }
        "replace" => {
            result.request_body = optional_string(object, "requestBody")?;
            result.response_body = optional_string(object, "responseBody")?;
            result.stream_chunk = optional_string(object, "streamChunk")?;
            result.log_message = optional_string(object, "logMessage")?;
            result.headers = optional_string_map(object, "headers")?.unwrap_or_default();
        }
        "appendMessage" => {
            result.request_body = Some(append_message_request_body(hook, context, object)?);
        }
        other => {
            return Err(GatewayPluginError::new(
                "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
                format!("unsupported extension host gateway action: {other}"),
            ));
        }
    }
    Ok(result)
}

fn hook_kind(hook: &str) -> Option<crate::gateway::plugins::contract::HookKind> {
    let hook_name = GatewayPluginHookName::from_str(hook)?;
    crate::gateway::plugins::registry::HookRegistry::new()
        .descriptor(hook_name)
        .map(|descriptor| descriptor.kind)
}

fn append_message_request_body(
    hook: &str,
    context: &GatewayVisibleHookContext,
    object: &serde_json::Map<String, Value>,
) -> Result<String, GatewayPluginError> {
    let hook_name = GatewayPluginHookName::from_str(hook).ok_or_else(|| {
        GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            format!("unknown gateway hook for appendMessage action: {hook}"),
        )
    })?;
    if !hook_name.is_request_hook() {
        return Err(GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            format!("appendMessage action is only allowed in request hooks: {hook}"),
        ));
    }
    let role = required_string(object, "role", "appendMessage action")?;
    if role != "system" && role != "developer" {
        return Err(GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            "appendMessage role must be system or developer",
        ));
    }
    let content = required_string(object, "content", "appendMessage action")?;
    if content.trim().is_empty() {
        return Err(GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            "appendMessage content must not be empty",
        ));
    }
    let body = context.request.body.as_deref().ok_or_else(|| {
        GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            "appendMessage requires visible request body",
        )
    })?;
    let mut root: Value = serde_json::from_str(body).map_err(|err| {
        GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            format!("appendMessage request body must be JSON: {err}"),
        )
    })?;
    let messages = root
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            GatewayPluginError::new(
                "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
                "appendMessage requires request body messages array",
            )
        })?;
    messages.push(json!({
        "role": role,
        "content": content,
    }));
    serde_json::to_string(&root).map_err(|err| {
        GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            format!("failed to encode appendMessage request body: {err}"),
        )
    })
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    key: &'static str,
    action: &'static str,
) -> Result<String, GatewayPluginError> {
    optional_string(object, key)?.ok_or_else(|| {
        GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            format!("{action} requires string {key}"),
        )
    })
}

fn optional_string(
    object: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Result<Option<String>, GatewayPluginError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            format!("extension host gateway output field {key} must be a string"),
        )),
    }
}

fn optional_string_map(
    object: &serde_json::Map<String, Value>,
    key: &'static str,
) -> Result<Option<BTreeMap<String, String>>, GatewayPluginError> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(map) = value.as_object() else {
        return Err(GatewayPluginError::new(
            "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
            format!("extension host gateway output field {key} must be an object"),
        ));
    };
    let mut out = BTreeMap::new();
    for (name, value) in map {
        let Some(header_value) = value.as_str() else {
            return Err(GatewayPluginError::new(
                "PLUGIN_EXTENSION_HOST_INVALID_OUTPUT",
                format!("extension host gateway output header {name} must be a string"),
            ));
        };
        out.insert(name.clone(), header_value.to_string());
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugins::{
        PluginDetail, PluginHostCompatibility, PluginInstallSource, PluginManifest,
        PluginPermissionRisk, PluginRuntime, PluginStatus, PluginSummary,
    };
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::{Duration, Instant};
    use tokio::sync::Notify;

    struct FakeExtensionHostFactory {
        state: Arc<StdMutex<FakeFactoryState>>,
    }

    #[derive(Default)]
    struct FakeFactoryState {
        next_id: u64,
        starts: Vec<u64>,
        start_timeouts: Vec<Duration>,
        executions: Vec<u64>,
        disposals: Vec<u64>,
    }

    struct FakeExtensionHostProcess {
        id: u64,
        state: Arc<StdMutex<FakeFactoryState>>,
        running: bool,
    }

    #[derive(Default)]
    struct BlockingExtensionHostFactory {
        slow_start: Arc<BlockingStartControl>,
        slow_command: Arc<BlockingCommandControl>,
        slow_dispose: Arc<BlockingDisposeControl>,
        starts: Arc<AtomicUsize>,
        disposals: Arc<AtomicUsize>,
    }

    #[derive(Default)]
    struct BlockingStartControl {
        started: Notify,
        release: Notify,
    }

    #[derive(Default)]
    struct BlockingCommandControl {
        starts: AtomicUsize,
        started: Notify,
        release: Notify,
    }

    #[derive(Default)]
    struct BlockingDisposeControl {
        started: Notify,
        release: Notify,
    }

    struct BlockingExtensionHostProcess {
        plugin_id: String,
        slow_command: Arc<BlockingCommandControl>,
        slow_dispose: Arc<BlockingDisposeControl>,
        disposals: Arc<AtomicUsize>,
        running: bool,
    }

    impl Default for FakeExtensionHostFactory {
        fn default() -> Self {
            Self {
                state: Arc::new(StdMutex::new(FakeFactoryState::default())),
            }
        }
    }

    impl FakeExtensionHostFactory {
        fn start_count(&self) -> usize {
            self.state.lock().unwrap().starts.len()
        }

        fn dispose_count(&self) -> usize {
            self.state.lock().unwrap().disposals.len()
        }

        fn executed_instance_ids(&self) -> Vec<u64> {
            self.state.lock().unwrap().executions.clone()
        }

        fn disposed_instance_ids(&self) -> Vec<u64> {
            self.state.lock().unwrap().disposals.clone()
        }

        fn start_timeouts(&self) -> Vec<Duration> {
            self.state.lock().unwrap().start_timeouts.clone()
        }
    }

    impl ExtensionHostFactory for FakeExtensionHostFactory {
        fn start<'a>(
            &'a self,
            _detail: PluginDetail,
            _db: db::Db,
            call_timeout: Duration,
        ) -> BoxFuture<'a, AppResult<Box<dyn ExtensionHostProcess>>> {
            Box::pin(async move {
                let mut state = self.state.lock().unwrap();
                state.next_id += 1;
                let id = state.next_id;
                state.starts.push(id);
                state.start_timeouts.push(call_timeout);
                Ok(Box::new(FakeExtensionHostProcess {
                    id,
                    state: self.state.clone(),
                    running: true,
                }) as Box<dyn ExtensionHostProcess>)
            })
        }
    }

    impl ExtensionHostProcess for FakeExtensionHostProcess {
        fn execute_command<'a>(
            &'a mut self,
            command: &'a str,
            args: Value,
        ) -> BoxFuture<'a, AppResult<Value>> {
            Box::pin(async move {
                if command == "fail-warm" {
                    return Err(AppError::new(
                        "PLUGIN_EXTENSION_CALL_TIMEOUT",
                        "warm command failed",
                    ));
                }
                self.state.lock().unwrap().executions.push(self.id);
                Ok(json!({
                    "instanceId": self.id,
                    "command": command,
                    "args": args,
                }))
            })
        }

        fn execute_gateway_hook<'a>(
            &'a mut self,
            hook: &'a str,
            context: Value,
        ) -> BoxFuture<'a, AppResult<Value>> {
            Box::pin(async move {
                if hook == "gateway.failWarm" {
                    return Err(AppError::new(
                        "PLUGIN_EXTENSION_CALL_TIMEOUT",
                        "warm gateway hook failed",
                    ));
                }
                self.state.lock().unwrap().executions.push(self.id);
                Ok(json!({
                    "action": "continue",
                    "hook": hook,
                    "context": context,
                }))
            })
        }

        fn is_running(&mut self) -> bool {
            self.running
        }

        fn dispose<'a>(&'a mut self) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                self.running = false;
                self.state.lock().unwrap().disposals.push(self.id);
            })
        }
    }

    impl ExtensionHostFactory for BlockingExtensionHostFactory {
        fn start<'a>(
            &'a self,
            detail: PluginDetail,
            _db: db::Db,
            _call_timeout: Duration,
        ) -> BoxFuture<'a, AppResult<Box<dyn ExtensionHostProcess>>> {
            Box::pin(async move {
                self.starts.fetch_add(1, Ordering::SeqCst);
                if detail.summary.plugin_id == "acme.start" {
                    self.slow_start.started.notify_waiters();
                    self.slow_start.release.notified().await;
                }
                Ok(Box::new(BlockingExtensionHostProcess {
                    plugin_id: detail.summary.plugin_id,
                    slow_command: self.slow_command.clone(),
                    slow_dispose: self.slow_dispose.clone(),
                    disposals: self.disposals.clone(),
                    running: true,
                }) as Box<dyn ExtensionHostProcess>)
            })
        }
    }

    impl ExtensionHostProcess for BlockingExtensionHostProcess {
        fn execute_command<'a>(
            &'a mut self,
            command: &'a str,
            _args: Value,
        ) -> BoxFuture<'a, AppResult<Value>> {
            Box::pin(async move {
                if (self.plugin_id == "acme.slow" && command == "slow")
                    || (self.plugin_id == "acme.race" && command == "race")
                {
                    self.slow_command.starts.fetch_add(1, Ordering::SeqCst);
                    self.slow_command.started.notify_waiters();
                    self.slow_command.release.notified().await;
                }
                Ok(json!({
                    "pluginId": self.plugin_id,
                    "command": command,
                }))
            })
        }

        fn execute_gateway_hook<'a>(
            &'a mut self,
            hook: &'a str,
            _context: Value,
        ) -> BoxFuture<'a, AppResult<Value>> {
            Box::pin(async move {
                Ok(json!({
                    "action": "continue",
                    "pluginId": self.plugin_id,
                    "hook": hook,
                }))
            })
        }

        fn is_running(&mut self) -> bool {
            self.running
        }

        fn dispose<'a>(&'a mut self) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                self.disposals.fetch_add(1, Ordering::SeqCst);
                if self.plugin_id == "acme.slow" {
                    self.slow_dispose.started.notify_waiters();
                    self.slow_dispose.release.notified().await;
                }
                self.running = false;
            })
        }
    }

    impl BlockingExtensionHostFactory {
        fn start_count(&self) -> usize {
            self.starts.load(Ordering::SeqCst)
        }

        fn dispose_count(&self) -> usize {
            self.disposals.load(Ordering::SeqCst)
        }

        fn command_start_count(&self) -> usize {
            self.slow_command.starts.load(Ordering::SeqCst)
        }

        async fn wait_for_command_start_count(&self, target: usize) {
            loop {
                let notified = self.slow_command.started.notified();
                if self.command_start_count() >= target {
                    return;
                }
                notified.await;
            }
        }
    }

    fn plugin_detail(plugin_id: &str, contribution_hash_seed: &str) -> PluginDetail {
        PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: plugin_id.to_string(),
                name: "Acme Echo".to_string(),
                current_version: Some("1.0.0".to_string()),
                status: PluginStatus::Enabled,
                runtime: "extensionHost".to_string(),
                permission_risk: PluginPermissionRisk::Low,
                update_available: false,
                last_error: None,
                created_at: 0,
                updated_at: 0,
            },
            manifest: PluginManifest {
                id: plugin_id.to_string(),
                name: "Acme Echo".to_string(),
                version: "1.0.0".to_string(),
                api_version: "1.0.0".to_string(),
                runtime: PluginRuntime::ExtensionHost {
                    language: "typescript".to_string(),
                },
                hooks: Vec::new(),
                permissions: Vec::new(),
                main: Some("dist/extension.js".to_string()),
                activation_events: Vec::new(),
                contributes: None,
                capabilities: vec![
                    "commands.execute".to_string(),
                    contribution_hash_seed.to_string(),
                ],
                host_compatibility: PluginHostCompatibility {
                    app: ">=0.60.0".to_string(),
                    plugin_api: "^1.0.0".to_string(),
                    platforms: Vec::new(),
                },
                entry: None,
                config_schema: None,
                config_version: None,
                description: None,
                author: None,
                homepage: None,
                repository: None,
                license: None,
                checksum: None,
                signature: None,
                category: None,
            },
            install_source: PluginInstallSource::Local,
            installed_dir: Some(format!("/tmp/{plugin_id}")),
            config: json!({}),
            granted_permissions: Vec::new(),
            pending_permissions: Vec::new(),
            audit_logs: Vec::new(),
            runtime_failures: Vec::new(),
            rollback_versions: Vec::new(),
        }
    }

    #[tokio::test]
    async fn registry_reuses_warm_instance_for_same_key() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        );
        let detail = plugin_detail("acme.echo", "same");

        let first = registry
            .execute_command_with_now(
                detail.clone(),
                "acme.echo",
                json!({ "n": 1 }),
                Instant::now(),
            )
            .await
            .expect("first command");
        let second = registry
            .execute_command_with_now(detail, "acme.echo", json!({ "n": 2 }), Instant::now())
            .await
            .expect("second command");

        assert!(first.cold_start);
        assert!(!second.cold_start);
        assert_eq!(factory.start_count(), 1);
        assert_eq!(factory.executed_instance_ids(), vec![1, 1]);
    }

    #[tokio::test]
    async fn registry_drops_warm_command_instance_after_execution_error() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        );
        let detail = plugin_detail("acme.echo", "same");

        registry
            .execute_command_with_now(detail.clone(), "warm", json!({}), Instant::now())
            .await
            .expect("first command warms instance");
        let err = registry
            .execute_command_with_now(detail.clone(), "fail-warm", json!({}), Instant::now())
            .await
            .expect_err("warm command error should propagate");

        assert_eq!(err.code(), "PLUGIN_EXTENSION_CALL_TIMEOUT");
        assert_eq!(registry.instance_count().await, 0);
        assert_eq!(factory.disposed_instance_ids(), vec![1]);

        let next = registry
            .execute_command_with_now(detail, "warm", json!({}), Instant::now())
            .await
            .expect("next command should cold start");

        assert!(next.cold_start);
        assert_eq!(factory.start_count(), 2);
        assert_eq!(factory.executed_instance_ids(), vec![1, 2]);
    }

    #[tokio::test]
    async fn registry_drops_warm_gateway_hook_instance_after_execution_error() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        );
        let detail = plugin_detail("acme.gateway", "same");
        let context = GatewayVisibleHookContext {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead
                .as_str()
                .to_string(),
            trace_id: "trace-gateway".to_string(),
            request: Default::default(),
            response: Default::default(),
            stream: Default::default(),
            log: Default::default(),
        };

        registry
            .execute_gateway_hook_with_now(
                detail.clone(),
                GatewayPluginHookName::RequestAfterBodyRead.as_str(),
                context.clone(),
                Duration::from_millis(150),
                Instant::now(),
            )
            .await
            .expect("first hook warms instance");
        let err = registry
            .execute_gateway_hook_with_now(
                detail.clone(),
                "gateway.failWarm",
                context.clone(),
                Duration::from_millis(150),
                Instant::now(),
            )
            .await
            .expect_err("warm gateway hook error should propagate");

        assert_eq!(err.code(), "PLUGIN_EXTENSION_HOST_TIMEOUT");
        assert_eq!(registry.instance_count().await, 0);
        assert_eq!(factory.disposed_instance_ids(), vec![1]);

        registry
            .execute_gateway_hook_with_now(
                detail,
                GatewayPluginHookName::RequestAfterBodyRead.as_str(),
                context,
                Duration::from_millis(150),
                Instant::now(),
            )
            .await
            .expect("next hook should cold start");

        assert_eq!(factory.start_count(), 2);
        assert_eq!(factory.executed_instance_ids(), vec![1, 2]);
    }

    #[tokio::test]
    async fn registry_starts_new_gateway_instance_when_call_timeout_changes() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        );
        let detail = plugin_detail("acme.gateway", "same");
        let context = GatewayVisibleHookContext {
            hook_name: GatewayPluginHookName::RequestAfterBodyRead
                .as_str()
                .to_string(),
            trace_id: "trace-gateway-timeout".to_string(),
            request: Default::default(),
            response: Default::default(),
            stream: Default::default(),
            log: Default::default(),
        };

        registry
            .execute_gateway_hook_with_now(
                detail.clone(),
                GatewayPluginHookName::RequestAfterBodyRead.as_str(),
                context.clone(),
                Duration::from_millis(37),
                Instant::now(),
            )
            .await
            .expect("first hook warms instance");
        registry
            .execute_gateway_hook_with_now(
                detail,
                GatewayPluginHookName::RequestAfterBodyRead.as_str(),
                context,
                Duration::from_millis(91),
                Instant::now(),
            )
            .await
            .expect("changed timeout should cold start a matching instance");

        assert_eq!(factory.start_count(), 2);
        assert_eq!(
            factory.start_timeouts(),
            vec![Duration::from_millis(37), Duration::from_millis(91)]
        );
        assert_eq!(factory.disposed_instance_ids(), vec![1]);
        assert_eq!(registry.instance_count().await, 1);
    }

    #[tokio::test]
    async fn registry_replaces_instance_when_contribution_hash_changes() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        );

        registry
            .execute_command_with_now(
                plugin_detail("acme.echo", "before"),
                "acme.echo",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("first command");
        let changed = registry
            .execute_command_with_now(
                plugin_detail("acme.echo", "after"),
                "acme.echo",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("changed command");

        assert!(changed.cold_start);
        assert_eq!(factory.start_count(), 2);
        assert_eq!(factory.dispose_count(), 1);
        assert_eq!(factory.disposed_instance_ids(), vec![1]);
    }

    #[tokio::test]
    async fn registry_disposes_plugin_instances() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        );

        registry
            .execute_command_with_now(
                plugin_detail("acme.echo", "one"),
                "acme.echo",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("first command");
        registry
            .execute_command_with_now(
                plugin_detail("acme.other", "two"),
                "acme.other",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("second command");

        registry.dispose_plugin("acme.echo").await;

        assert_eq!(factory.disposed_instance_ids(), vec![1]);
        assert_eq!(registry.instance_count().await, 1);
    }

    #[tokio::test]
    async fn registry_retain_plugins_disposes_removed_plugin_instances() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        );

        registry
            .execute_command_with_now(
                plugin_detail("acme.keep", "one"),
                "acme.keep",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("first command");
        registry
            .execute_command_with_now(
                plugin_detail("acme.remove", "two"),
                "acme.remove",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("second command");

        registry
            .retain_plugins(&HashSet::from(["acme.keep".to_string()]))
            .await;

        assert_eq!(factory.disposed_instance_ids(), vec![2]);
        assert_eq!(registry.plugin_instance_count("acme.keep").await, 1);
        assert_eq!(registry.plugin_instance_count("acme.remove").await, 0);
    }

    #[tokio::test]
    async fn registry_disposes_idle_instances() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(10),
            },
        );
        let now = Instant::now();

        registry
            .execute_command_with_now(
                plugin_detail("acme.echo", "idle"),
                "acme.echo",
                json!({}),
                now,
            )
            .await
            .expect("first command");
        registry.dispose_idle(now + Duration::from_secs(11)).await;

        assert_eq!(factory.disposed_instance_ids(), vec![1]);
        assert_eq!(registry.instance_count().await, 0);
    }

    #[tokio::test]
    async fn registry_evicts_least_recently_used_idle_instance() {
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 2,
                idle_recycle: Duration::from_secs(120),
            },
        );
        let now = Instant::now();

        registry
            .execute_command_with_now(plugin_detail("acme.one", "one"), "acme.one", json!({}), now)
            .await
            .expect("first command");
        registry
            .execute_command_with_now(
                plugin_detail("acme.two", "two"),
                "acme.two",
                json!({}),
                now + Duration::from_secs(1),
            )
            .await
            .expect("second command");
        registry
            .execute_command_with_now(
                plugin_detail("acme.one", "one"),
                "acme.one",
                json!({}),
                now + Duration::from_secs(2),
            )
            .await
            .expect("touch first command");
        registry
            .execute_command_with_now(
                plugin_detail("acme.three", "three"),
                "acme.three",
                json!({}),
                now + Duration::from_secs(3),
            )
            .await
            .expect("third command");

        assert_eq!(factory.disposed_instance_ids(), vec![2]);
        assert_eq!(registry.instance_count().await, 2);
    }

    #[tokio::test]
    async fn registry_allows_different_plugin_commands_while_one_plugin_command_is_slow() {
        let factory = Arc::new(BlockingExtensionHostFactory::default());
        let registry = Arc::new(ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        ));
        let slow_registry = registry.clone();

        let slow_task = tokio::spawn(async move {
            slow_registry
                .execute_command_with_now(
                    plugin_detail("acme.slow", "slow"),
                    "slow",
                    json!({}),
                    Instant::now(),
                )
                .await
        });
        tokio::time::timeout(
            Duration::from_secs(1),
            factory.slow_command.started.notified(),
        )
        .await
        .expect("slow command should start");

        let fast_result = tokio::time::timeout(
            Duration::from_millis(100),
            registry.execute_command_with_now(
                plugin_detail("acme.fast", "fast"),
                "fast",
                json!({}),
                Instant::now(),
            ),
        )
        .await;

        factory.slow_command.release.notify_waiters();
        slow_task
            .await
            .expect("slow task join")
            .expect("slow command result");

        assert!(
            fast_result.is_ok(),
            "fast plugin command should not wait for slow plugin command"
        );
        fast_result
            .expect("fast command should complete")
            .expect("fast command result");
    }

    #[tokio::test]
    async fn registry_dispose_plugin_does_not_block_other_plugin_commands() {
        let factory = Arc::new(BlockingExtensionHostFactory::default());
        let registry = Arc::new(ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        ));
        registry
            .execute_command_with_now(
                plugin_detail("acme.slow", "slow"),
                "warm",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("warm slow plugin");
        registry
            .execute_command_with_now(
                plugin_detail("acme.fast", "fast"),
                "warm",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("warm fast plugin");

        let dispose_registry = registry.clone();
        let dispose_task = tokio::spawn(async move {
            dispose_registry.dispose_plugin("acme.slow").await;
        });
        tokio::time::timeout(
            Duration::from_secs(1),
            factory.slow_dispose.started.notified(),
        )
        .await
        .expect("slow dispose should start");

        let fast_result = tokio::time::timeout(
            Duration::from_millis(100),
            registry.execute_command_with_now(
                plugin_detail("acme.fast", "fast"),
                "fast",
                json!({}),
                Instant::now(),
            ),
        )
        .await;

        factory.slow_dispose.release.notify_waiters();
        dispose_task.await.expect("dispose task join");

        assert!(
            fast_result.is_ok(),
            "fast plugin command should not wait for slow plugin dispose"
        );
        fast_result
            .expect("fast command should complete")
            .expect("fast command result");
    }

    #[tokio::test]
    async fn registry_serializes_same_plugin_replacement_during_concurrent_cold_start() {
        let factory = Arc::new(BlockingExtensionHostFactory::default());
        let registry = Arc::new(ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        ));
        let first_registry = registry.clone();
        let second_registry = registry.clone();

        let first_task = tokio::spawn(async move {
            first_registry
                .execute_command_with_now(
                    plugin_detail("acme.race", "before"),
                    "race",
                    json!({}),
                    Instant::now(),
                )
                .await
        });
        factory.wait_for_command_start_count(1).await;

        let second_task = tokio::spawn(async move {
            second_registry
                .execute_command_with_now(
                    plugin_detail("acme.race", "after"),
                    "race",
                    json!({}),
                    Instant::now(),
                )
                .await
        });
        let second_started_while_first_running = tokio::time::timeout(
            Duration::from_millis(100),
            factory.wait_for_command_start_count(2),
        )
        .await;

        factory.slow_command.release.notify_waiters();
        factory.wait_for_command_start_count(2).await;
        factory.slow_command.release.notify_waiters();
        first_task
            .await
            .expect("first task join")
            .expect("first command result");
        second_task
            .await
            .expect("second task join")
            .expect("second command result");

        assert!(
            second_started_while_first_running.is_err(),
            "same plugin replacement should wait for the first execution to finish"
        );
        assert_eq!(factory.start_count(), 2);
        assert_eq!(factory.dispose_count(), 1);
        assert_eq!(registry.plugin_instance_count("acme.race").await, 1);
        assert_eq!(registry.instance_count().await, 1);
    }

    #[tokio::test]
    async fn registry_dispose_all_waits_for_in_flight_cold_start_before_returning() {
        let factory = Arc::new(BlockingExtensionHostFactory::default());
        let registry = Arc::new(ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        ));
        let command_registry = registry.clone();

        let command_task = tokio::spawn(async move {
            command_registry
                .execute_command_with_now(
                    plugin_detail("acme.start", "start"),
                    "start",
                    json!({}),
                    Instant::now(),
                )
                .await
        });
        tokio::time::timeout(
            Duration::from_secs(1),
            factory.slow_start.started.notified(),
        )
        .await
        .expect("cold start should begin");

        let dispose_registry = registry.clone();
        let dispose_task = tokio::spawn(async move {
            dispose_registry.dispose_all().await;
        });
        let dispose_finished_before_start_released =
            tokio::time::timeout(Duration::from_millis(100), async {
                while !dispose_task.is_finished() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await;

        factory.slow_start.release.notify_waiters();
        command_task
            .await
            .expect("command task join")
            .expect("command result");
        dispose_task.await.expect("dispose_all task join");

        assert!(
            dispose_finished_before_start_released.is_err(),
            "dispose_all should wait for in-flight cold start to finish"
        );
        assert_eq!(registry.instance_count().await, 0);
    }

    #[tokio::test]
    async fn runtime_state_dispose_plugin_if_initialized_is_noop_before_registry_init() {
        let state = ExtensionHostRuntimeState::default();

        state.dispose_plugin_if_initialized("acme.echo").await;
    }

    #[tokio::test]
    async fn runtime_state_dispose_plugin_if_initialized_disposes_existing_registry_instance() {
        let state = ExtensionHostRuntimeState::default();
        let factory = Arc::new(FakeExtensionHostFactory::default());
        let registry = Arc::new(ExtensionHostInstanceRegistry::new_for_tests(
            factory.clone(),
            ExtensionHostRegistryLimits {
                max_warm_instances: 8,
                idle_recycle: Duration::from_secs(120),
            },
        ));
        state.set_registry_for_tests(registry.clone()).await;

        registry
            .execute_command_with_now(
                plugin_detail("acme.echo", "dispose"),
                "acme.echo",
                json!({}),
                Instant::now(),
            )
            .await
            .expect("execute command");

        state.dispose_plugin_if_initialized("acme.echo").await;

        assert_eq!(factory.disposed_instance_ids(), vec![1]);
        assert_eq!(registry.instance_count().await, 0);
    }
}
