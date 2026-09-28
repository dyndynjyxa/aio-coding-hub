use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PROVIDER_MODEL_POLICY_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderModelPolicyStatus {
    Legacy,
    Ready,
    Invalid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderModelMode {
    All,
    Selected,
    Excluded,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderModelMapping {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderModelPolicyV1 {
    pub version: u32,
    pub mode: ProviderModelMode,
    pub model_patterns: Vec<String>,
    pub mappings: Vec<ProviderModelMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderModelEligibility {
    Blocked,
    Explicit,
    Fallback,
}

impl ProviderModelPolicyV1 {
    pub fn all() -> Self {
        Self {
            version: PROVIDER_MODEL_POLICY_VERSION,
            mode: ProviderModelMode::All,
            model_patterns: Vec::new(),
            mappings: Vec::new(),
        }
    }

    pub fn normalized(mut self) -> Result<Self, String> {
        if self.version != PROVIDER_MODEL_POLICY_VERSION {
            return Err(format!(
                "SEC_INVALID_INPUT: unsupported provider model policy version {}",
                self.version
            ));
        }
        let mut seen_patterns = HashSet::with_capacity(self.model_patterns.len());
        for pattern in &mut self.model_patterns {
            *pattern = pattern.trim().to_string();
            validate_policy_value("model pattern", pattern)?;
            if !seen_patterns.insert(pattern.clone()) {
                return Err(
                    "SEC_INVALID_INPUT: provider model policy pattern is duplicated".into(),
                );
            }
        }

        let mut seen_mappings = HashSet::with_capacity(self.mappings.len());
        for mapping in &mut self.mappings {
            mapping.source = mapping.source.trim().to_string();
            mapping.target = mapping.target.trim().to_string();
            validate_policy_value("mapping source", &mapping.source)?;
            validate_policy_value("mapping target", &mapping.target)?;
            if !mapping.source.contains('*') && mapping.target.contains('*') {
                return Err("SEC_INVALID_INPUT: target wildcard requires a source wildcard".into());
            }
            if !seen_mappings.insert(mapping.source.clone()) {
                return Err(
                    "SEC_INVALID_INPUT: provider model policy mapping source is duplicated".into(),
                );
            }
        }

        if self.mode == ProviderModelMode::Selected
            && seen_patterns.is_empty()
            && seen_mappings.is_empty()
        {
            return Err(
                "SEC_INVALID_INPUT: selected provider model policy needs a pattern or mapping"
                    .into(),
            );
        }

        self.model_patterns
            .sort_by(|left, right| compare_patterns(left, right));
        self.mappings
            .sort_by(|left, right| compare_patterns(&left.source, &right.source));
        Ok(self)
    }

    pub fn decode(raw: Option<&str>, cli_key: &str) -> (Option<Self>, ProviderModelPolicyStatus) {
        let Some(raw) = raw else {
            let status = if cli_key == "claude" {
                ProviderModelPolicyStatus::Legacy
            } else {
                ProviderModelPolicyStatus::Invalid
            };
            return (None, status);
        };

        match serde_json::from_str::<Self>(raw)
            .map_err(|error| error.to_string())
            .and_then(Self::normalized)
        {
            Ok(policy) => (Some(policy), ProviderModelPolicyStatus::Ready),
            Err(error) => {
                tracing::warn!(cli_key, error = %error, "provider model policy decode failed");
                (None, ProviderModelPolicyStatus::Invalid)
            }
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| format!("SYSTEM_ERROR: {error}"))
    }

    pub(crate) fn eligibility(&self, source_model: &str) -> ProviderModelEligibility {
        let pattern_matches = self
            .model_patterns
            .iter()
            .any(|pattern| match_pattern(pattern, source_model).is_some());
        if self.mode == ProviderModelMode::Excluded && pattern_matches {
            return ProviderModelEligibility::Blocked;
        }
        if pattern_matches
            || self
                .mappings
                .iter()
                .any(|mapping| match_pattern(&mapping.source, source_model).is_some())
        {
            return ProviderModelEligibility::Explicit;
        }
        if self.mode == ProviderModelMode::Selected {
            ProviderModelEligibility::Blocked
        } else {
            ProviderModelEligibility::Fallback
        }
    }

    pub fn resolve_mapping(&self, source_model: &str) -> String {
        for mapping in &self.mappings {
            let Some(capture) = match_pattern(&mapping.source, source_model) else {
                continue;
            };
            return mapping.target.replace('*', capture);
        }
        source_model.to_string()
    }

    pub(crate) fn has_mapping_match(&self, source_model: &str) -> bool {
        self.mappings
            .iter()
            .any(|mapping| match_pattern(&mapping.source, source_model).is_some())
    }
}

fn validate_policy_value(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!(
            "SEC_INVALID_INPUT: provider model policy {field} is required"
        ));
    }
    if value.matches('*').count() > 1 {
        return Err(format!(
            "SEC_INVALID_INPUT: provider model policy {field} supports at most one wildcard"
        ));
    }
    Ok(())
}

pub(crate) fn normalize_concrete_model_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("SEC_INVALID_INPUT: model id is required".into());
    }
    if value.contains('*') {
        return Err("SEC_INVALID_INPUT: model id cannot contain wildcard".into());
    }
    Ok(value.to_string())
}

fn compare_patterns(left: &str, right: &str) -> std::cmp::Ordering {
    left.contains('*')
        .cmp(&right.contains('*'))
        .then_with(|| {
            right
                .chars()
                .filter(|character| *character != '*')
                .count()
                .cmp(&left.chars().filter(|character| *character != '*').count())
        })
        .then_with(|| left.cmp(right))
}

fn match_pattern<'a>(pattern: &'a str, source_model: &'a str) -> Option<&'a str> {
    let Some(star) = pattern.find('*') else {
        return (pattern == source_model).then_some("");
    };
    let prefix = &pattern[..star];
    let suffix = &pattern[star + 1..];
    let remainder = source_model.strip_prefix(prefix)?;
    let capture = remainder.strip_suffix(suffix)?;
    Some(capture)
}

#[cfg(test)]
mod tests {
    use super::{
        ProviderModelEligibility, ProviderModelMapping, ProviderModelMode,
        ProviderModelPolicyStatus, ProviderModelPolicyV1,
    };

    fn mapping(source: &str, target: &str) -> ProviderModelMapping {
        ProviderModelMapping {
            source: source.to_string(),
            target: target.to_string(),
        }
    }

    fn policy(
        mode: ProviderModelMode,
        model_patterns: Vec<&str>,
        mappings: Vec<ProviderModelMapping>,
    ) -> ProviderModelPolicyV1 {
        ProviderModelPolicyV1 {
            version: 1,
            mode,
            model_patterns: model_patterns.into_iter().map(str::to_string).collect(),
            mappings,
        }
    }

    #[test]
    fn provider_model_policy_normalizes_and_validates_entries() {
        let normalized = policy(
            ProviderModelMode::All,
            vec!["  gpt-5.4  "],
            vec![mapping("  gpt-*  ", "  upstream-*  ")],
        )
        .normalized()
        .expect("valid policy");
        assert_eq!(normalized.model_patterns, vec!["gpt-5.4"]);
        assert_eq!(normalized.mappings, vec![mapping("gpt-*", "upstream-*")]);

        let invalid = [
            policy(ProviderModelMode::Selected, vec![], vec![]),
            policy(ProviderModelMode::All, vec![""], vec![]),
            policy(ProviderModelMode::All, vec!["gpt-*", " gpt-* "], vec![]),
            policy(ProviderModelMode::All, vec!["gpt-*-*"], vec![]),
            policy(
                ProviderModelMode::All,
                vec![],
                vec![mapping("gpt", "upstream-*")],
            ),
            policy(ProviderModelMode::All, vec![], vec![mapping("gpt", "")]),
        ];
        for value in invalid {
            assert!(value.normalized().is_err());
        }
    }

    #[test]
    fn provider_model_policy_separates_eligibility_and_mapping() {
        let value = policy(
            ProviderModelMode::Selected,
            vec!["claude-*"],
            vec![
                mapping("gpt-*", "broad-*"),
                mapping("gpt-5.*", "specific-*"),
                mapping("gpt-5.4", "exact"),
                mapping("gpt-5.*-mini", "vendor-*"),
            ],
        )
        .normalized()
        .expect("valid policy");

        assert_eq!(
            value.eligibility("gpt-5.4"),
            ProviderModelEligibility::Explicit
        );
        assert_eq!(value.resolve_mapping("gpt-5.4"), "exact");
        assert_eq!(value.resolve_mapping("gpt-5.4-mini"), "vendor-4");
        assert_eq!(value.resolve_mapping("gpt-5.3"), "specific-3");
        assert_eq!(
            value.eligibility("claude-sonnet"),
            ProviderModelEligibility::Explicit
        );
        assert_eq!(value.resolve_mapping("claude-sonnet"), "claude-sonnet");
        assert_eq!(
            value.eligibility("gemini-2.5"),
            ProviderModelEligibility::Blocked
        );

        let non_recursive = policy(
            ProviderModelMode::Selected,
            vec![],
            vec![mapping("source", "gpt-5.4"), mapping("gpt-5.4", "final")],
        )
        .normalized()
        .expect("valid policy");
        assert_eq!(non_recursive.resolve_mapping("source"), "gpt-5.4");
    }

    #[test]
    fn provider_model_policy_exclusion_wins_over_mapping() {
        let value = policy(
            ProviderModelMode::Excluded,
            vec!["gpt-5.6-*"],
            vec![mapping("gpt-5.6-luna", "deepseek-v4-flash")],
        )
        .normalized()
        .expect("valid policy");

        assert_eq!(
            value.eligibility("gpt-5.6-luna"),
            ProviderModelEligibility::Blocked
        );
        assert_eq!(
            value.eligibility("gpt-5.5"),
            ProviderModelEligibility::Fallback
        );
    }

    #[test]
    fn provider_model_policy_rejects_unpublished_wire_shape() {
        let (decoded, status) = ProviderModelPolicyV1::decode(
            Some(
                r#"{"version":1,"mode":"selected","rules":[{"source":"gpt-5.6-luna","target":"deepseek-v4-flash"},{"source":"gpt-5.5","target":null}]}"#,
            ),
            "codex",
        );
        assert_eq!(status, ProviderModelPolicyStatus::Invalid);
        assert_eq!(decoded, None);

        let policy = policy(ProviderModelMode::All, vec!["gpt-5.6-luna"], vec![]);
        let json = policy.to_json().expect("serialize current policy");
        assert!(json.contains(r#""modelPatterns""#));
        assert!(json.contains(r#""mappings""#));
        assert!(!json.contains(r#""rules""#));
    }

    #[test]
    fn provider_model_policy_accepts_large_catalogs_and_long_unicode_ids() {
        let long_source = format!("model-{}", "模".repeat(201));
        let long_target = format!("upstream-{}", "型".repeat(201));
        let model_patterns = (0..501).map(|index| format!("model-{index}")).collect();
        let value = ProviderModelPolicyV1 {
            version: 1,
            mode: ProviderModelMode::Selected,
            model_patterns,
            mappings: vec![mapping(&long_source, &long_target)],
        }
        .normalized()
        .expect("large policy should be valid");

        assert_eq!(value.model_patterns.len(), 501);
        assert_eq!(value.resolve_mapping(&long_source), long_target);
        assert_eq!(
            super::normalize_concrete_model_id(&long_source),
            Ok(long_source)
        );
    }

    #[test]
    fn provider_model_policy_null_is_legacy_only_for_claude() {
        let (claude_policy, claude_status) = ProviderModelPolicyV1::decode(None, "claude");
        assert_eq!(claude_policy, None);
        assert_eq!(claude_status, ProviderModelPolicyStatus::Legacy);

        let (codex_policy, codex_status) = ProviderModelPolicyV1::decode(None, "codex");
        assert_eq!(codex_policy, None);
        assert_eq!(codex_status, ProviderModelPolicyStatus::Invalid);
    }
}
