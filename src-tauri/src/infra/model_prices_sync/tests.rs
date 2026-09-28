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

#[test]
fn decode_basellm_cache_keeps_validators_only_for_current_parser_version() {
    // Current-version cache round-trips through the writer path untouched.
    let mut headers = HeaderMap::new();
    headers.insert(reqwest::header::ETAG, HeaderValue::from_static("\"e1\""));
    headers.insert(LAST_MODIFIED, HeaderValue::from_static("lm"));
    let json = serde_json::to_string(&headers_to_cache(&headers)).expect("serialize cache");
    let decoded = decode_basellm_cache(&json);
    assert_eq!(decoded.etag.as_deref(), Some("\"e1\""));
    assert_eq!(decoded.last_modified.as_deref(), Some("lm"));

    // A cache written by an older parser (no/stale version) must not suppress
    // a full re-fetch: its validators are dropped.
    let legacy = r#"{"etag":"\"e1\"","last_modified":"lm"}"#;
    let decoded = decode_basellm_cache(legacy);
    assert_eq!(decoded.etag, None);
    assert_eq!(decoded.last_modified, None);

    // Corrupt content behaves like no cache at all.
    assert_eq!(decode_basellm_cache("not json").etag, None);
}

#[test]
fn failed_report_carries_error_and_failed_status() {
    let report = failed_report("source unavailable".to_string());
    assert_eq!(report.status, ModelPricesSyncStatus::Failed);
    assert_eq!(report.error.as_deref(), Some("source unavailable"));
    assert_eq!(report.total, 0);
}

#[test]
fn writes_each_price_row_once_under_its_provider_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = db::init_for_tests(&dir.path().join("model-price-single-copy.db")).expect("init db");
    let counts = upsert_rows(
        &db,
        vec![ModelPriceRow {
            cli_key: "claude",
            vendor: "deepseek".to_string(),
            model: "deepseek-v4-flash".to_string(),
            price_json:
                r#"{"input_cost_per_token":"0.00000014","output_cost_per_token":"0.00000028"}"#
                    .to_string(),
        }],
    )
    .expect("upsert prices");

    assert_eq!(counts.inserted, 1);
    let conn = db.open_connection().expect("open db");
    let copies: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM model_prices WHERE model = 'deepseek-v4-flash'",
            [],
            |row| row.get(0),
        )
        .expect("count copies");
    assert_eq!(copies, 1);
    drop(conn);
    let rows = crate::model_prices::list_all(&db).expect("list prices");
    assert!(rows.iter().any(|row| row.cli_key == "claude"
        && row.vendor == "deepseek"
        && row.model == "deepseek-v4-flash"));
}

#[test]
fn maps_third_party_basellm_providers_to_claude_cli() {
    let root = serde_json::json!({
      "deepseek": {
        "models": { "deepseek-v4-flash": { "cost": { "input": 0.14, "output": 0.28 } } }
      },
      "minimax": {
        "models": { "MiniMax-M3": { "cost": { "input": 0.3, "output": 1.2 } } }
      },
      "unsupported-provider": {
        "models": { "mystery": { "cost": { "input": 1, "output": 2 } } }
      }
    });

    let rows = parse_basellm_all_json(&root).expect("rows");

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.cli_key == "claude"));
    assert!(rows
        .iter()
        .any(|row| row.vendor == "deepseek" && row.model == "deepseek-v4-flash"));
    assert!(rows
        .iter()
        .any(|row| row.vendor == "minimax" && row.model == "MiniMax-M3"));
}

#[test]
fn maps_non_200k_context_tier_to_custom_threshold_fields() {
    // Mirrors basellm's MiniMax-M3 shape: a 512k context tier plus a mirrored
    // `context_over_200k` whose 200k threshold semantics would be wrong.
    let root = serde_json::json!({
      "minimax": {
        "models": {
          "MiniMax-M3": {
            "cost": {
              "input": 0.3, "output": 1.2, "cache_read": 0.06,
              "context_over_200k": { "input": 0.6, "output": 2.4, "cache_read": 0.12 },
              "tiers": [
                { "input": 0.6, "output": 2.4, "cache_read": 0.12,
                  "tier": { "size": 512000, "type": "context" } }
              ]
            }
          }
        }
      }
    });

    let rows = parse_basellm_all_json(&root).expect("rows");
    assert_eq!(rows.len(), 1);
    let price: Value = serde_json::from_str(&rows[0].price_json).expect("price json");

    assert_eq!(
        price
            .get("context_tier_threshold_tokens")
            .and_then(Value::as_i64),
        Some(512000)
    );
    assert_eq!(
        price
            .get("input_cost_per_token_above_threshold")
            .and_then(Value::as_str),
        Some("0.0000006")
    );
    assert_eq!(
        price
            .get("output_cost_per_token_above_threshold")
            .and_then(Value::as_str),
        Some("0.0000024")
    );
    assert_eq!(
        price
            .get("cache_read_input_token_cost_above_threshold")
            .and_then(Value::as_str),
        Some("0.00000012")
    );
    assert!(price
        .get("input_cost_per_token_above_200k_tokens")
        .is_none());
}

#[test]
fn keeps_legacy_200k_fields_for_200k_context_tier() {
    let root = serde_json::json!({
      "google": {
        "models": {
          "gemini-2.5-pro": {
            "cost": {
              "input": 1.25, "output": 10,
              "context_over_200k": { "input": 2.5, "output": 15 },
              "tiers": [
                { "input": 2.5, "output": 15, "tier": { "size": 200000, "type": "context" } }
              ]
            }
          }
        }
      }
    });

    let rows = parse_basellm_all_json(&root).expect("rows");
    assert_eq!(rows.len(), 1);
    let price: Value = serde_json::from_str(&rows[0].price_json).expect("price json");

    assert!(price.get("context_tier_threshold_tokens").is_none());
    assert_eq!(
        price
            .get("input_cost_per_token_above_200k_tokens")
            .and_then(Value::as_str),
        Some("0.0000025")
    );
}

#[test]
fn upsert_reports_unchanged_when_price_and_vendor_match() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = db::init_for_tests(&dir.path().join("model-price-unchanged.db")).expect("init db");
    let row = || ModelPriceRow {
        cli_key: "claude",
        vendor: "moonshotai".to_string(),
        model: "kimi-k3".to_string(),
        price_json: r#"{"input_cost_per_token":"0.000003"}"#.to_string(),
    };
    let first = upsert_rows(&db, vec![row()]).expect("insert");
    assert_eq!((first.inserted, first.updated, first.unchanged), (1, 0, 0));

    // Same price + vendor: unchanged. A vendor change alone counts as updated.
    let second = upsert_rows(&db, vec![row()]).expect("re-upsert");
    assert_eq!(
        (second.inserted, second.updated, second.unchanged),
        (0, 0, 1)
    );

    let mut moved = row();
    moved.vendor = "deepseek".to_string();
    let third = upsert_rows(&db, vec![moved]).expect("vendor change");
    assert_eq!((third.inserted, third.updated, third.unchanged), (0, 1, 0));
}
