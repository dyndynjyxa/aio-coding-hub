use crate::db;
use rusqlite::Connection;

use super::folders::{
    folder_options_from_rows, resolved_folder_map, session_lookup_keys, usage_event_rows,
};
use super::{
    resolve_query_params, UsageFolderOptionV1, UsageQueryParams, UsageResolvedFolder,
    UsageSessionLookupKey,
};

pub(super) fn folder_options_v1_with_conn<F>(
    conn: &Connection,
    params: &UsageQueryParams,
    folder_lookup: F,
) -> Result<Vec<UsageFolderOptionV1>, String>
where
    F: FnOnce(&[UsageSessionLookupKey]) -> Vec<UsageResolvedFolder>,
{
    let resolved = resolve_query_params(conn, params)?;
    let rows = usage_event_rows(
        conn,
        resolved.start_ts,
        resolved.end_ts,
        resolved.cli_key,
        resolved.provider_id,
        None,
        false,
        resolved.exclude_cx2cc_gateway_bridge,
    )?;
    let lookup_keys = session_lookup_keys(&rows);
    let folder_map = resolved_folder_map(folder_lookup(&lookup_keys));
    Ok(folder_options_from_rows(&rows, &folder_map))
}

pub fn folder_options_v1<F>(
    db: &db::Db,
    params: &UsageQueryParams,
    folder_lookup: F,
) -> crate::shared::error::AppResult<Vec<UsageFolderOptionV1>>
where
    F: FnOnce(&[UsageSessionLookupKey]) -> Vec<UsageResolvedFolder>,
{
    let conn = db.open_connection()?;
    Ok(folder_options_v1_with_conn(&conn, params, folder_lookup)?)
}
