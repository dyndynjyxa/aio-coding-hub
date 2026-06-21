use super::resolve_session_bound_provider_id;
use crate::circuit_breaker;
use crate::{providers, session_manager};
use std::collections::HashMap;

fn ids(items: &[providers::ProviderForGateway]) -> Vec<i64> {
    items.iter().map(|p| p.id).collect()
}

fn insert_provider(db: &crate::db::Db, name: &str, enabled: bool) -> providers::ProviderSummary {
    let provider = providers::upsert(
        db,
        providers::ProviderUpsertParams {
            provider_id: None,
            cli_key: "claude".to_string(),
            name: name.to_string(),
            base_urls: vec!["https://example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            auth_mode: None,
            api_key: Some("k".to_string()),
            enabled,
            cost_multiplier: 1.0,
            priority: Some(100),
            claude_models: None,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: Some(providers::DailyResetMode::Fixed),
            daily_reset_time: Some("00:00:00".to_string()),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: None,
            note: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
        },
    )
    .expect("insert provider");

    let mut provider_ids: Vec<i64> = providers::default_route_list(db, "claude")
        .expect("list default route")
        .into_iter()
        .map(|row| row.provider_id)
        .collect();
    provider_ids.push(provider.id);
    providers::default_route_set_order(db, "claude", provider_ids)
        .expect("append default route provider");

    provider
}

fn insert_sort_mode_with_providers(db: &crate::db::Db, provider_ids: &[i64]) -> i64 {
    let conn = db.open_connection().expect("open db");
    conn.execute(
        "INSERT INTO sort_modes(name, created_at, updated_at) VALUES (?1, ?2, ?3)",
        rusqlite::params!["Mode A", 1000, 1000],
    )
    .expect("insert mode");
    let mode_id = conn.last_insert_rowid();
    for (idx, provider_id) in provider_ids.iter().enumerate() {
        conn.execute(
            r#"
INSERT INTO sort_mode_providers(
  mode_id,
  cli_key,
  provider_id,
  sort_order,
  enabled,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
"#,
            rusqlite::params![mode_id, "claude", provider_id, idx as i64, 1, 1000, 1000],
        )
        .expect("insert mode provider");
    }
    mode_id
}

fn open_circuit_for_provider(provider_id: i64, now: i64) -> circuit_breaker::CircuitBreaker {
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration_secs: 3600,
        },
        HashMap::new(),
        None,
    );
    circuit.record_failure(provider_id, now);
    assert!(!circuit.should_allow(provider_id, now).allow);
    circuit
}

#[test]
fn resolve_session_bound_provider_id_skips_disabled_bound_provider() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    providers::set_enabled(&db, id1, false).expect("disable provider 1");

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    let mut enabled =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    assert_eq!(ids(&enabled), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        None,
        &mut enabled,
        Some(&order),
    );

    // Disabled provider must NOT be re-inserted; fall through to next enabled provider
    assert_eq!(selected, None);
    assert_eq!(ids(&enabled), vec![id2]);
}

#[test]
fn resolve_session_bound_provider_id_skips_insertion_when_forced_provider_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    providers::set_enabled(&db, id1, false).expect("disable provider 1");

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    let mut enabled =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    assert_eq!(ids(&enabled), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        Some(id2),
        &mut enabled,
        Some(&order),
    );

    assert_eq!(selected, None);
    assert_eq!(ids(&enabled), vec![id2]);
}

#[test]
fn resolve_session_bound_provider_id_does_not_insert_when_reuse_disabled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    providers::set_enabled(&db, id1, false).expect("disable provider 1");

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    let mut enabled =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    assert_eq!(ids(&enabled), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        false,
        None,
        &mut enabled,
        Some(&order),
    );

    assert_eq!(selected, None);
    assert_eq!(ids(&enabled), vec![id2]);
}

#[test]
fn resolve_session_bound_provider_id_clears_stale_binding_when_bound_provider_not_in_candidates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let id1 = p1.id;
    let id2 = p2.id;

    let session = session_manager::SessionManager::new();
    let circuit = circuit_breaker::CircuitBreaker::new(
        circuit_breaker::CircuitBreakerConfig::default(),
        HashMap::new(),
        None,
    );
    let now = 1000;
    session.bind_success("claude", "sess_1", id1, None, now);

    // Simulate a mode/provider list that no longer contains the bound provider.
    let mut candidates =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    candidates.retain(|p| p.id == id2);
    assert_eq!(ids(&candidates), vec![id2]);

    let order = vec![id1, id2];
    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        None,
        &mut candidates,
        Some(&order),
    );

    // Must NOT re-insert the stale provider; reuse should fall through.
    assert_eq!(selected, None);
    assert_eq!(ids(&candidates), vec![id2]);
    assert_eq!(session.get_bound_provider("claude", "sess_1", now), None);
}

#[test]
fn default_mode_switches_to_enabled_provider_after_bound_provider_disabled_and_circuit_open() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let now = 1000;
    let session = session_manager::SessionManager::new();
    session.bind_success("claude", "sess_1", p1.id, None, now);
    let circuit = open_circuit_for_provider(p1.id, now);

    providers::set_enabled(&db, p1.id, false).expect("disable provider 1 globally");

    let mut enabled =
        providers::list_enabled_for_gateway_in_mode(&db, "claude", None).expect("list enabled");
    assert_eq!(ids(&enabled), vec![p2.id]);

    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        None,
        &mut enabled,
        Some(&[p1.id, p2.id]),
    );

    assert_eq!(ids(&enabled), vec![p2.id]);
    assert_eq!(selected, None);
    assert_eq!(session.get_bound_provider("claude", "sess_1", now), None);
}

#[test]
fn sort_mode_ignores_global_provider_enabled_but_open_circuit_prevents_session_reuse() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::init_for_tests(&db_path).expect("init db");

    let p1 = insert_provider(&db, "P1", true);
    let p2 = insert_provider(&db, "P2", true);
    let mode_id = insert_sort_mode_with_providers(&db, &[p1.id, p2.id]);
    let now = 1000;
    let session = session_manager::SessionManager::new();
    session.bind_success("claude", "sess_1", p1.id, Some(mode_id), now);
    let circuit = open_circuit_for_provider(p1.id, now);

    providers::set_enabled(&db, p1.id, false).expect("disable provider 1 globally");

    let mut enabled = providers::list_enabled_for_gateway_in_mode(&db, "claude", Some(mode_id))
        .expect("list enabled");
    assert_eq!(ids(&enabled), vec![p1.id, p2.id]);

    let selected = resolve_session_bound_provider_id(
        &session,
        &circuit,
        "claude",
        Some("sess_1"),
        now,
        true,
        None,
        &mut enabled,
        Some(&[p1.id, p2.id]),
    );

    assert_eq!(ids(&enabled), vec![p1.id, p2.id]);
    assert_eq!(selected, None);
    assert_eq!(
        session.get_bound_provider("claude", "sess_1", now),
        Some(p1.id)
    );
    assert!(!circuit.should_allow(p1.id, now).allow);
    assert!(circuit.should_allow(p2.id, now).allow);
}
