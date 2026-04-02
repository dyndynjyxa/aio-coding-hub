pub(super) fn token_total(total: Option<i64>, input: Option<i64>, output: Option<i64>) -> i64 {
    if let Some(t) = total {
        return t;
    }
    input.unwrap_or(0).saturating_add(output.unwrap_or(0))
}

pub(super) const SQL_EFFECTIVE_INPUT_TOKENS_EXPR: &str = "CASE WHEN cli_key IN ('codex','gemini') OR EXISTS (SELECT 1 FROM providers p WHERE p.id = final_provider_id AND p.source_provider_id IS NOT NULL) THEN MAX(COALESCE(input_tokens, 0) - COALESCE(cache_read_input_tokens, 0), 0) ELSE COALESCE(input_tokens, 0) END";

pub(super) fn sql_effective_input_tokens_expr_with_alias(alias: &str) -> String {
    format!(
        "CASE WHEN {alias}.cli_key IN ('codex','gemini') OR EXISTS (SELECT 1 FROM providers p WHERE p.id = {alias}.final_provider_id AND p.source_provider_id IS NOT NULL) THEN MAX(COALESCE({alias}.input_tokens, 0) - COALESCE({alias}.cache_read_input_tokens, 0), 0) ELSE COALESCE({alias}.input_tokens, 0) END"
    )
}

pub(super) fn sql_effective_total_tokens_expr() -> String {
    format!(
        "({effective_input_expr}) + COALESCE(output_tokens, 0) + COALESCE(cache_creation_input_tokens, 0) + COALESCE(cache_read_input_tokens, 0)",
        effective_input_expr = SQL_EFFECTIVE_INPUT_TOKENS_EXPR
    )
}

pub(super) fn sql_effective_total_tokens_expr_with_alias(alias: &str) -> String {
    let effective_input_expr = sql_effective_input_tokens_expr_with_alias(alias);
    format!(
        "({effective_input_expr}) + COALESCE({alias}.output_tokens, 0) + COALESCE({alias}.cache_creation_input_tokens, 0) + COALESCE({alias}.cache_read_input_tokens, 0)",
    )
}
