use super::*;

#[test]
fn shifts_1m_cost_to_per_token_with_plain_number() {
    assert_eq!(
        shift_cost_per_1m_to_per_token("2.5").as_deref(),
        Some("0.0000025")
    );
    assert_eq!(
        shift_cost_per_1m_to_per_token("10").as_deref(),
        Some("0.00001")
    );
}

#[test]
fn shifts_1m_cost_to_per_token_with_existing_exponent() {
    assert_eq!(
        shift_cost_per_1m_to_per_token("1e3").as_deref(),
        Some("0.001")
    );
    assert_eq!(
        shift_cost_per_1m_to_per_token("1e-3").as_deref(),
        Some("0.000000001")
    );
}

#[test]
fn parses_google_context_over_200k_to_above_200k_fields() {
    let root = serde_json::json!({
      "google": {
        "models": {
          "gemini-3-pro-preview": {
            "cost": {
              "input": 2,
              "output": 10,
              "context_over_200k": { "input": 3, "output": 15 }
            }
          }
        }
      }
    });

    let rows = parse_basellm_all_json(&root).expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cli_key, "gemini");
    assert_eq!(rows[0].model, "gemini-3-pro-preview");

    let price: Value = serde_json::from_str(&rows[0].price_json).expect("price json");
    assert_eq!(
        price
            .get("input_cost_per_token")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "0.000002"
    );
    assert_eq!(
        price
            .get("output_cost_per_token")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "0.00001"
    );
    assert_eq!(
        price
            .get("input_cost_per_token_above_200k_tokens")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "0.000003"
    );
    assert_eq!(
        price
            .get("output_cost_per_token_above_200k_tokens")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "0.000015"
    );
}

#[test]
fn parses_xai_text_model_with_complete_token_costs_as_grok() {
    let root = serde_json::json!({
      "xai": {
        "models": {
          "grok-build-0.1": {
            "modalities": { "input": ["text", "image"], "output": ["text"] },
            "cost": { "input": 1, "output": 2, "cache_read": 0.2 }
          }
        }
      }
    });

    let rows = parse_basellm_all_json(&root).expect("rows");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cli_key, "grok");
    assert_eq!(rows[0].model, "grok-build-0.1");
    let price: Value = serde_json::from_str(&rows[0].price_json).expect("price json");
    assert_eq!(
        price.get("input_cost_per_token").and_then(Value::as_str),
        Some("0.000001")
    );
    assert_eq!(
        price.get("output_cost_per_token").and_then(Value::as_str),
        Some("0.000002")
    );
}

#[test]
fn skips_model_without_complete_input_and_output_costs() {
    let root = serde_json::json!({
      "openai": {
        "models": {
          "input-only": {
            "modalities": { "output": ["text"] },
            "cost": { "input": 1 }
          },
          "output-only": {
            "modalities": { "output": ["text"] },
            "cost": { "output": 2 }
          }
        }
      }
    });

    let rows = parse_basellm_all_json(&root).expect("rows");

    assert!(rows.is_empty());
}

#[test]
fn skips_model_with_declared_non_text_output() {
    let root = serde_json::json!({
      "openai": {
        "models": {
          "grok-imagine-priced": {
            "modalities": { "input": ["text"], "output": ["image"] },
            "cost": { "input": 1, "output": 2 }
          }
        }
      }
    });

    let rows = parse_basellm_all_json(&root).expect("rows");

    assert!(rows.is_empty());
}

#[test]
fn write_json_atomically_rejects_oversized_basellm_cache_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("basellm-cache.json");

    let err = write_json_atomically(&path, vec![b'x'; BASELLM_CACHE_MAX_BYTES + 1])
        .unwrap_err()
        .to_string();

    assert!(err.contains("basellm cache file too large"));
    assert!(!path.exists());
}
