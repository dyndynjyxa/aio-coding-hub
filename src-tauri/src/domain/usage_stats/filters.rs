//! Shared SQL WHERE-clause builders for usage analytics queries.

pub(super) type SqlValues = Vec<rusqlite::types::Value>;

/// Build optional AND clauses for time-range, cli_key, and provider_id filters.
///
/// Returns `("\nAND col >= ?1\nAND col < ?2\n...", values)` or `("", [])` when
/// no filters apply. Column names are caller-supplied so the same builder works
/// across tables with different schemas.
pub(super) fn build_optional_range_cli_provider_filters(
    created_at_column: &str,
    cli_key_column: &str,
    provider_id_column: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    cli_key: Option<&str>,
    provider_id: Option<i64>,
) -> (String, SqlValues) {
    let mut clauses = Vec::new();
    let mut values: SqlValues = Vec::with_capacity(4);

    if let Some(ts) = start_ts {
        values.push(ts.into());
        clauses.push(format!("{created_at_column} >= ?{}", values.len()));
    }

    if let Some(ts) = end_ts {
        values.push(ts.into());
        clauses.push(format!("{created_at_column} < ?{}", values.len()));
    }

    if let Some(cli) = cli_key {
        values.push(cli.to_string().into());
        clauses.push(format!("{cli_key_column} = ?{}", values.len()));
    }

    if let Some(id) = provider_id {
        values.push(id.into());
        clauses.push(format!("{provider_id_column} = ?{}", values.len()));
    }

    let sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("\nAND {}", clauses.join("\nAND "))
    };

    (sql, values)
}

pub(super) fn sql_exclude_cx2cc_gateway_bridge_clause(
    request_log_alias: Option<&str>,
    enabled: bool,
) -> String {
    if !enabled {
        return String::new();
    }

    let final_provider_id_column = match request_log_alias {
        Some(alias) if !alias.trim().is_empty() => format!("{}.final_provider_id", alias.trim()),
        _ => "final_provider_id".to_string(),
    };

    format!(
        r#"
AND NOT EXISTS (
  SELECT 1
  FROM providers usage_filter_provider
  WHERE usage_filter_provider.id = {final_provider_id_column}
  AND usage_filter_provider.bridge_type = 'cx2cc'
  AND usage_filter_provider.source_provider_id IS NULL
)"#
    )
}

/// Build optional AND clauses for time-range filters with a placeholder offset.
///
/// Similar to [`build_optional_range_cli_provider_filters`] but only handles
/// `start_ts`/`end_ts` and numbers placeholders starting at `placeholder_offset + 1`.
pub(super) fn build_optional_range_filters_with_offset(
    created_at_column: &str,
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    placeholder_offset: usize,
) -> (String, SqlValues) {
    let mut clauses = Vec::new();
    let mut values: SqlValues = Vec::with_capacity(2);

    if let Some(ts) = start_ts {
        values.push(ts.into());
        clauses.push(format!(
            "{created_at_column} >= ?{}",
            placeholder_offset + values.len()
        ));
    }

    if let Some(ts) = end_ts {
        values.push(ts.into());
        clauses.push(format!(
            "{created_at_column} < ?{}",
            placeholder_offset + values.len()
        ));
    }

    let sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("\nAND {}", clauses.join("\nAND "))
    };

    (sql, values)
}
