//! Usage: Model price alias rules (filesystem JSON config).
//!
//! This is a lightweight, user-configurable mapping layer used by request log cost calculation
//! to resolve model name mismatches (e.g. `claude-opus-4-5-thinking` -> `claude-opus-4-5`).

use crate::app_paths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MODEL_PRICE_DIR_NAME: &str = "model-prices";
const ALIASES_FILE_NAME: &str = "price-aliases.json";
const ALIASES_SCHEMA_VERSION_V1: i64 = 1;
const MAX_MODEL_LEN: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPriceAliasMatchTypeV1 {
    Exact,
    Prefix,
    Wildcard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPriceAliasRuleV1 {
    pub cli_key: String,
    pub match_type: ModelPriceAliasMatchTypeV1,
    pub pattern: String,
    pub target_model: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelPriceAliasesV1 {
    pub version: i64,
    pub rules: Vec<ModelPriceAliasRuleV1>,
}

impl Default for ModelPriceAliasesV1 {
    fn default() -> Self {
        Self {
            version: ALIASES_SCHEMA_VERSION_V1,
            rules: Vec::new(),
        }
    }
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    match cli_key {
        "claude" | "codex" | "gemini" => Ok(()),
        _ => Err(format!("SEC_INVALID_INPUT: unknown cli_key={cli_key}")),
    }
}

fn model_prices_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app_paths::app_data_dir(app)?.join(MODEL_PRICE_DIR_NAME);
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create model-prices dir: {e}"))?;
    Ok(dir)
}

fn aliases_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(model_prices_dir(app)?.join(ALIASES_FILE_NAME))
}

fn sanitize_nonempty_trimmed(input: &str, field: &'static str) -> Result<String, String> {
    let value = input.trim();
    if value.is_empty() {
        return Err(format!("SEC_INVALID_INPUT: {field} is required"));
    }
    if value.len() > MAX_MODEL_LEN {
        return Err(format!(
            "SEC_INVALID_INPUT: {field} is too long (max {MAX_MODEL_LEN})"
        ));
    }
    Ok(value.to_string())
}

fn validate_wildcard_pattern(pattern: &str) -> Result<(), String> {
    let count = pattern.chars().filter(|c| *c == '*').count();
    if count != 1 {
        return Err("SEC_INVALID_INPUT: wildcard pattern must contain exactly one '*'".to_string());
    }
    Ok(())
}

fn validate_rule(mut rule: ModelPriceAliasRuleV1) -> Result<ModelPriceAliasRuleV1, String> {
    let cli_key = rule.cli_key.trim().to_ascii_lowercase();
    validate_cli_key(&cli_key)?;
    rule.cli_key = cli_key;

    rule.pattern = sanitize_nonempty_trimmed(&rule.pattern, "pattern")?;
    rule.target_model = sanitize_nonempty_trimmed(&rule.target_model, "target_model")?;

    if rule.target_model.contains('*') {
        return Err("SEC_INVALID_INPUT: target_model must not contain '*'".to_string());
    }

    match rule.match_type {
        ModelPriceAliasMatchTypeV1::Exact | ModelPriceAliasMatchTypeV1::Prefix => {
            if rule.pattern.contains('*') {
                return Err(
                    "SEC_INVALID_INPUT: pattern must not contain '*' for exact/prefix rules"
                        .to_string(),
                );
            }
        }
        ModelPriceAliasMatchTypeV1::Wildcard => validate_wildcard_pattern(&rule.pattern)?,
    }

    Ok(rule)
}

fn validate_aliases(mut aliases: ModelPriceAliasesV1) -> Result<ModelPriceAliasesV1, String> {
    if aliases.version != ALIASES_SCHEMA_VERSION_V1 {
        return Err(format!(
            "SEC_INVALID_INPUT: unsupported aliases version {}",
            aliases.version
        ));
    }

    let mut out: Vec<ModelPriceAliasRuleV1> = Vec::with_capacity(aliases.rules.len());
    for rule in aliases.rules {
        out.push(validate_rule(rule)?);
    }
    aliases.rules = out;
    Ok(aliases)
}

fn write_json_atomically(path: &Path, json_bytes: Vec<u8>) -> Result<(), String> {
    let tmp_path = path.with_extension("json.tmp");
    let backup_path = path.with_extension("json.bak");

    std::fs::write(&tmp_path, json_bytes)
        .map_err(|e| format!("failed to write temp aliases file: {e}"))?;

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    if path.exists() {
        std::fs::rename(path, &backup_path)
            .map_err(|e| format!("failed to create aliases backup: {e}"))?;
    }

    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::rename(&backup_path, path);
        return Err(format!("failed to finalize aliases file: {e}"));
    }

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    Ok(())
}

pub fn read_fail_open(app: &tauri::AppHandle) -> ModelPriceAliasesV1 {
    match read(app) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("model_price_aliases read error: {err}");
            ModelPriceAliasesV1::default()
        }
    }
}

pub fn read(app: &tauri::AppHandle) -> Result<ModelPriceAliasesV1, String> {
    let path = aliases_path(app)?;
    if !path.exists() {
        return Ok(ModelPriceAliasesV1::default());
    }

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("failed to read aliases: {e}"))?;
    let parsed: ModelPriceAliasesV1 =
        serde_json::from_str(&content).map_err(|e| format!("failed to parse aliases: {e}"))?;
    validate_aliases(parsed)
}

pub fn write(
    app: &tauri::AppHandle,
    aliases: ModelPriceAliasesV1,
) -> Result<ModelPriceAliasesV1, String> {
    let aliases = validate_aliases(aliases)?;
    let path = aliases_path(app)?;
    let bytes = serde_json::to_vec_pretty(&aliases)
        .map_err(|e| format!("failed to serialize aliases: {e}"))?;
    write_json_atomically(&path, bytes)?;
    Ok(aliases)
}

fn match_wildcard_single(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() != 2 {
        return false;
    }
    let prefix = parts[0];
    let suffix = parts[1];
    text.starts_with(prefix) && text.ends_with(suffix)
}

fn match_rule(rule: &ModelPriceAliasRuleV1, model: &str) -> bool {
    match rule.match_type {
        ModelPriceAliasMatchTypeV1::Exact => rule.pattern == model,
        ModelPriceAliasMatchTypeV1::Prefix => model.starts_with(rule.pattern.as_str()),
        ModelPriceAliasMatchTypeV1::Wildcard => match_wildcard_single(rule.pattern.as_str(), model),
    }
}

fn match_type_rank(match_type: &ModelPriceAliasMatchTypeV1) -> u8 {
    match match_type {
        ModelPriceAliasMatchTypeV1::Exact => 0,
        ModelPriceAliasMatchTypeV1::Wildcard => 1,
        ModelPriceAliasMatchTypeV1::Prefix => 2,
    }
}

impl ModelPriceAliasesV1 {
    pub fn resolve_target_model<'a>(
        &'a self,
        cli_key: &str,
        requested_model: &str,
    ) -> Option<&'a str> {
        let requested_model = requested_model.trim();
        if requested_model.is_empty() {
            return None;
        }
        if requested_model.len() > MAX_MODEL_LEN {
            return None;
        }

        let cli_key = cli_key.trim();
        if validate_cli_key(cli_key).is_err() {
            return None;
        }

        let mut matches: Vec<&ModelPriceAliasRuleV1> = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if rule.cli_key != cli_key {
                continue;
            }
            if match_rule(rule, requested_model) {
                matches.push(rule);
            }
        }
        if matches.is_empty() {
            return None;
        }

        // Deterministic selection: match type rank, then longer patterns, then lexicographic.
        matches.sort_by(|a, b| {
            match_type_rank(&a.match_type)
                .cmp(&match_type_rank(&b.match_type))
                .then_with(|| b.pattern.len().cmp(&a.pattern.len()))
                .then_with(|| a.pattern.cmp(&b.pattern))
                .then_with(|| a.target_model.cmp(&b.target_model))
        });

        Some(matches[0].target_model.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_single_star_matches_prefix_suffix() {
        assert!(match_wildcard_single("a*b", "axxb"));
        assert!(match_wildcard_single("*b", "b"));
        assert!(match_wildcard_single("a*", "a"));
        assert!(!match_wildcard_single("a*b", "abx"));
        assert!(!match_wildcard_single("a*b*c", "abc"));
    }

    #[test]
    fn resolves_exact_over_wildcard_over_prefix() {
        let aliases = ModelPriceAliasesV1 {
            version: 1,
            rules: vec![
                ModelPriceAliasRuleV1 {
                    cli_key: "gemini".to_string(),
                    match_type: ModelPriceAliasMatchTypeV1::Prefix,
                    pattern: "gemini-3".to_string(),
                    target_model: "gemini-3-any".to_string(),
                    enabled: true,
                },
                ModelPriceAliasRuleV1 {
                    cli_key: "gemini".to_string(),
                    match_type: ModelPriceAliasMatchTypeV1::Wildcard,
                    pattern: "gemini-3-*".to_string(),
                    target_model: "gemini-3-wild".to_string(),
                    enabled: true,
                },
                ModelPriceAliasRuleV1 {
                    cli_key: "gemini".to_string(),
                    match_type: ModelPriceAliasMatchTypeV1::Exact,
                    pattern: "gemini-3-flash".to_string(),
                    target_model: "gemini-3-flash-preview".to_string(),
                    enabled: true,
                },
            ],
        };

        assert_eq!(
            aliases.resolve_target_model("gemini", "gemini-3-flash"),
            Some("gemini-3-flash-preview")
        );
        assert_eq!(
            aliases.resolve_target_model("gemini", "gemini-3-pro"),
            Some("gemini-3-wild")
        );
    }

    #[test]
    fn resolves_longer_patterns_first_within_same_type() {
        let aliases = ModelPriceAliasesV1 {
            version: 1,
            rules: vec![
                ModelPriceAliasRuleV1 {
                    cli_key: "claude".to_string(),
                    match_type: ModelPriceAliasMatchTypeV1::Prefix,
                    pattern: "claude-opus".to_string(),
                    target_model: "a".to_string(),
                    enabled: true,
                },
                ModelPriceAliasRuleV1 {
                    cli_key: "claude".to_string(),
                    match_type: ModelPriceAliasMatchTypeV1::Prefix,
                    pattern: "claude-opus-4-5".to_string(),
                    target_model: "b".to_string(),
                    enabled: true,
                },
            ],
        };

        assert_eq!(
            aliases.resolve_target_model("claude", "claude-opus-4-5-thinking"),
            Some("b")
        );
    }
}
