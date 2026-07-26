use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cx2ccCostBasis {
    pub(crate) bridge_provider_id: Option<i64>,
    pub(crate) source_cli_key: String,
    pub(crate) priced_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cx2ccCostBasisResolution {
    Matched(Cx2ccCostBasis),
    ScopedMismatch,
    Missing,
}

impl Cx2ccCostBasisResolution {
    pub(crate) fn openai_input_semantics_override(&self) -> Option<bool> {
        match self {
            Self::Matched(basis) => Some(basis.source_cli_key == "codex"),
            Self::ScopedMismatch => Some(false),
            Self::Missing => None,
        }
    }
}

pub(crate) fn resolve_cx2cc_cost_basis(
    special_settings_json: Option<&str>,
    final_provider_id: Option<i64>,
) -> Cx2ccCostBasisResolution {
    let Some(raw) = special_settings_json.map(trim_json_whitespace) else {
        return Cx2ccCostBasisResolution::Missing;
    };
    if raw.is_empty() {
        return Cx2ccCostBasisResolution::Missing;
    }

    let Ok(settings) = serde_json::from_str::<Vec<Value>>(raw) else {
        return Cx2ccCostBasisResolution::Missing;
    };
    let mut legacy = None;
    let mut has_scoped_marker = false;

    for setting in settings.iter().rev() {
        let Some(basis) = parse_cost_basis(setting) else {
            continue;
        };
        match basis.bridge_provider_id {
            Some(provider_id) => {
                has_scoped_marker = true;
                if Some(provider_id) == final_provider_id {
                    return Cx2ccCostBasisResolution::Matched(basis);
                }
            }
            None if legacy.is_none() => legacy = Some(basis),
            None => {}
        }
    }

    if has_scoped_marker {
        Cx2ccCostBasisResolution::ScopedMismatch
    } else if let Some(basis) = legacy {
        Cx2ccCostBasisResolution::Matched(basis)
    } else {
        Cx2ccCostBasisResolution::Missing
    }
}

fn parse_cost_basis(setting: &Value) -> Option<Cx2ccCostBasis> {
    let obj = setting.as_object()?;
    if obj.get("type").and_then(Value::as_str) != Some("cx2cc_cost_basis") {
        return None;
    }

    let source_cli_key = obj
        .get("source_cli_key")
        .and_then(Value::as_str)
        .map(trim_json_whitespace)
        .filter(|value| !value.is_empty())?;
    let priced_model = obj
        .get("priced_model")
        .and_then(Value::as_str)
        .map(trim_json_whitespace)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let bridge_provider_id = match obj.get("bridge_provider_id") {
        None => None,
        Some(value) => Some(value.as_i64().filter(|provider_id| *provider_id > 0)?),
    };

    Some(Cx2ccCostBasis {
        bridge_provider_id,
        source_cli_key: source_cli_key.to_string(),
        priced_model,
    })
}

fn trim_json_whitespace(value: &str) -> &str {
    value.trim_matches(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_last_valid_legacy_marker_and_keeps_model_optional() {
        let json = serde_json::json!([
            "ignored",
            {
                "type": "cx2cc_cost_basis",
                "source_cli_key": "claude",
                "priced_model": "claude-opus"
            },
            {
                "type": "cx2cc_cost_basis",
                "source_cli_key": "\tcodex\r\n"
            },
            {
                "type": "cx2cc_cost_basis",
                "source_cli_key": "   ",
                "priced_model": "ignored"
            }
        ])
        .to_string();

        assert_eq!(
            resolve_cx2cc_cost_basis(Some(&json), Some(99)),
            Cx2ccCostBasisResolution::Matched(Cx2ccCostBasis {
                bridge_provider_id: None,
                source_cli_key: "codex".to_string(),
                priced_model: None,
            })
        );
    }

    #[test]
    fn scoped_markers_bind_semantics_to_the_final_provider() {
        let json = serde_json::json!([
            {
                "type": "cx2cc_cost_basis",
                "bridge_provider_id": 7,
                "source_cli_key": "codex",
                "priced_model": "gpt-5.6"
            },
            {
                "type": "cx2cc_cost_basis",
                "source_cli_key": "codex",
                "priced_model": "legacy-gpt"
            },
            {
                "type": "cx2cc_cost_basis",
                "bridge_provider_id": 8,
                "source_cli_key": "claude",
                "priced_model": "claude-opus"
            }
        ])
        .to_string();

        assert!(matches!(
            resolve_cx2cc_cost_basis(Some(&json), Some(7)),
            Cx2ccCostBasisResolution::Matched(Cx2ccCostBasis {
                bridge_provider_id: Some(7),
                source_cli_key,
                ..
            }) if source_cli_key == "codex"
        ));
        assert!(matches!(
            resolve_cx2cc_cost_basis(Some(&json), Some(8)),
            Cx2ccCostBasisResolution::Matched(Cx2ccCostBasis {
                bridge_provider_id: Some(8),
                source_cli_key,
                ..
            }) if source_cli_key == "claude"
        ));
        assert_eq!(
            resolve_cx2cc_cost_basis(Some(&json), Some(9)),
            Cx2ccCostBasisResolution::ScopedMismatch,
            "a scoped marker from a failed attempt must block legacy/provider fallback"
        );
    }

    #[test]
    fn rejects_missing_malformed_or_invalidly_scoped_markers() {
        for raw in [
            None,
            Some(""),
            Some("not-json"),
            Some("{}"),
            Some(r#"[{"type":"other","source_cli_key":"codex"}]"#),
            Some(r#"[{"type":"cx2cc_cost_basis"}]"#),
            Some(
                r#"[{"type":"cx2cc_cost_basis","source_cli_key":"codex","bridge_provider_id":0}]"#,
            ),
            Some(
                r#"[{"type":"cx2cc_cost_basis","source_cli_key":"codex","bridge_provider_id":null}]"#,
            ),
        ] {
            assert_eq!(
                resolve_cx2cc_cost_basis(raw, Some(1)),
                Cx2ccCostBasisResolution::Missing,
                "raw={raw:?}"
            );
        }
    }
}
