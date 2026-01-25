pub(super) fn token_total(total: Option<i64>, input: Option<i64>, output: Option<i64>) -> i64 {
    if let Some(t) = total {
        return t;
    }
    input.unwrap_or(0).saturating_add(output.unwrap_or(0))
}

fn is_cache_read_subset_cli(cli_key: &str) -> bool {
    matches!(cli_key, "codex" | "gemini")
}

pub(super) fn effective_input_tokens(
    cli_key: &str,
    raw_input_tokens: i64,
    cache_read_input_tokens: i64,
) -> i64 {
    let raw_input_tokens = raw_input_tokens.max(0);
    let cache_read_input_tokens = cache_read_input_tokens.max(0);

    if is_cache_read_subset_cli(cli_key) {
        (raw_input_tokens.saturating_sub(cache_read_input_tokens)).max(0)
    } else {
        raw_input_tokens
    }
}

pub(super) fn effective_total_tokens(
    effective_input_tokens: i64,
    output_tokens: i64,
    cache_creation_input_tokens: i64,
    cache_read_input_tokens: i64,
) -> i64 {
    effective_input_tokens
        .max(0)
        .saturating_add(output_tokens.max(0))
        .saturating_add(cache_creation_input_tokens.max(0))
        .saturating_add(cache_read_input_tokens.max(0))
}

pub(super) const SQL_EFFECTIVE_INPUT_TOKENS_EXPR: &str = "CASE WHEN cli_key IN ('codex','gemini') THEN MAX(COALESCE(input_tokens, 0) - COALESCE(cache_read_input_tokens, 0), 0) ELSE COALESCE(input_tokens, 0) END";

pub(super) fn sql_effective_total_tokens_expr() -> String {
    format!(
        "({effective_input_expr}) + COALESCE(output_tokens, 0) + COALESCE(cache_creation_input_tokens, 0) + COALESCE(cache_read_input_tokens, 0)",
        effective_input_expr = SQL_EFFECTIVE_INPUT_TOKENS_EXPR
    )
}
