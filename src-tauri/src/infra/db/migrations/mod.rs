//! Usage: SQLite schema migrations (user_version + incremental upgrades).

mod baseline_v25;
mod ensure;
mod v25_to_v26;
mod v26_to_v27;
mod v27_to_v28;
mod v28_to_v29;
mod v29_to_v30;
mod v30_to_v31;
mod v31_to_v32;
mod v32_to_v33;
mod v33_to_v34;
mod v34_to_v35;
mod v35_to_v36;
mod v36_to_v37;
mod v37_to_v38;
mod v38_to_v39;

use rusqlite::Connection;

const LATEST_SCHEMA_VERSION: i64 = 39;
const MAX_COMPAT_SCHEMA_VERSION: i64 = 39;
const MIN_SUPPORTED_SCHEMA_VERSION: i64 = 25;

pub(super) fn apply_migrations(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let mut user_version = read_user_version(conn)?;

    if user_version < 0 || (user_version > 0 && user_version < MIN_SUPPORTED_SCHEMA_VERSION) {
        return Err(format!(
            "unsupported sqlite schema version: user_version={user_version} (minimum supported: {MIN_SUPPORTED_SCHEMA_VERSION}, upgrade from an earlier app version first)"
        )
        .into());
    }

    if user_version > MAX_COMPAT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported sqlite schema version: user_version={user_version} (expected {MIN_SUPPORTED_SCHEMA_VERSION}..={MAX_COMPAT_SCHEMA_VERSION})"
        )
        .into());
    }

    let start_version = user_version;

    // Fresh install: create complete schema at v25
    if user_version == 0 {
        baseline_v25::create_baseline_v25(conn)?;
        user_version = read_user_version(conn)?;
        tracing::info!(to_version = user_version, "sqlite baseline schema created");
    }

    // Incremental migrations from v25 to v30
    while user_version < LATEST_SCHEMA_VERSION {
        let from_version = user_version;
        match user_version {
            25 => v25_to_v26::migrate_v25_to_v26(conn)?,
            26 => v26_to_v27::migrate_v26_to_v27(conn)?,
            27 => v27_to_v28::migrate_v27_to_v28(conn)?,
            28 => v28_to_v29::migrate_v28_to_v29(conn)?,
            29 => v29_to_v30::migrate_v29_to_v30(conn)?,
            30 => v30_to_v31::migrate_v30_to_v31(conn)?,
            31 => v31_to_v32::migrate_v31_to_v32(conn)?,
            32 => v32_to_v33::migrate_v32_to_v33(conn)?,
            33 => v33_to_v34::migrate_v33_to_v34(conn)?,
            34 => v34_to_v35::migrate_v34_to_v35(conn)?,
            35 => v35_to_v36::migrate_v35_to_v36(conn)?,
            36 => v36_to_v37::migrate_v36_to_v37(conn)?,
            37 => v37_to_v38::migrate_v37_to_v38(conn)?,
            38 => v38_to_v39::migrate_v38_to_v39(conn)?,
            v => {
                tracing::error!(
                    version = v,
                    "unsupported sqlite schema version during migration"
                );
                return Err(format!(
                    "unsupported sqlite schema version: user_version={v} (expected {MIN_SUPPORTED_SCHEMA_VERSION}..={MAX_COMPAT_SCHEMA_VERSION})"
                )
                .into());
            }
        }
        user_version = read_user_version(conn)?;
        tracing::info!(
            from_version = from_version,
            to_version = user_version,
            "sqlite migration step completed"
        );
    }

    if start_version < user_version {
        tracing::info!(
            from_version = start_version,
            to_version = user_version,
            "sqlite migrations completed"
        );
    }

    // Idempotent ensure patches (always run)
    ensure::apply_ensure_patches(conn)?;

    // Normalize dev builds back to LATEST_SCHEMA_VERSION
    let user_version = read_user_version(conn)?;
    if user_version > LATEST_SCHEMA_VERSION {
        let tx = conn
            .transaction()
            .map_err(|e| format!("failed to start sqlite transaction: {e}"))?;
        set_user_version(&tx, LATEST_SCHEMA_VERSION)?;
        tx.commit()
            .map_err(|e| format!("failed to commit sqlite transaction: {e}"))?;
    }

    Ok(())
}

pub(super) fn apply_runtime_ensure_patches(
    conn: &mut Connection,
) -> crate::shared::error::AppResult<()> {
    ensure::apply_ensure_patches(conn)
}

fn read_user_version(conn: &Connection) -> crate::shared::error::AppResult<i64> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| format!("failed to read sqlite user_version: {e}").into())
}

pub(super) fn set_user_version(
    tx: &rusqlite::Transaction<'_>,
    version: i64,
) -> crate::shared::error::AppResult<()> {
    tx.pragma_update(None, "user_version", version)
        .map_err(|e| format!("failed to update sqlite user_version: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests;
