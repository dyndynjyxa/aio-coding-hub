//! Input validation and normalization for provider operations.

use std::collections::HashSet;

pub(super) const MAX_LIMIT_USD: f64 = 1_000_000_000.0;
pub(super) const MAX_STREAM_IDLE_TIMEOUT_SECONDS: u32 = 60 * 60;
pub(super) const MAX_PROVIDER_BASE_URLS: usize = 32;
pub(super) const MAX_PROVIDER_BASE_URL_CHARS: usize = 2048;
pub(super) const MAX_PROVIDER_NOTE_CHARS: usize = 500;
pub(super) const MAX_PROVIDER_ORDER_IDS: usize = 512;

pub(super) fn exceeds_max_chars(value: &str, max_chars: usize) -> bool {
    value.chars().nth(max_chars).is_some()
}

pub(super) fn validate_max_chars(
    field: &str,
    value: &str,
    max_chars: usize,
) -> crate::shared::error::AppResult<()> {
    if exceeds_max_chars(value, max_chars) {
        return Err(
            format!("SEC_INVALID_INPUT: {field} must be at most {max_chars} characters").into(),
        );
    }
    Ok(())
}

pub(super) fn normalize_note(value: Option<&str>) -> crate::shared::error::AppResult<String> {
    let note = value.unwrap_or("").trim().to_string();
    validate_max_chars("note", &note, MAX_PROVIDER_NOTE_CHARS)?;
    Ok(note)
}

/// Write-path validation: rejects over-length model names, matching the frontend
/// editor limit. The read path (`normalize_model_slot`) keeps truncating so
/// legacy/hand-edited rows still load.
pub(super) fn validate_claude_models(
    models: &super::types::ClaudeModels,
) -> crate::shared::error::AppResult<()> {
    let fields = [
        ("main_model", models.main_model.as_deref()),
        ("reasoning_model", models.reasoning_model.as_deref()),
        ("haiku_model", models.haiku_model.as_deref()),
        ("sonnet_model", models.sonnet_model.as_deref()),
        ("opus_model", models.opus_model.as_deref()),
    ];
    for (field, value) in fields {
        let trimmed = value.unwrap_or("").trim();
        if trimmed.is_empty() {
            continue;
        }
        validate_max_chars(field, trimmed, super::types::MAX_MODEL_NAME_LEN)?;
    }
    Ok(())
}

pub(super) fn parse_reset_time_hms(input: &str) -> Option<(u8, u8, u8)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split(':');
    let h = parts.next()?;
    let m = parts.next()?;
    let s = parts.next();
    if parts.next().is_some() {
        return None;
    }

    if !(1..=2).contains(&h.len()) {
        return None;
    }
    if m.len() != 2 {
        return None;
    }
    if let Some(sec) = s {
        if sec.len() != 2 {
            return None;
        }
    }

    let hours: u8 = h.parse().ok()?;
    let minutes: u8 = m.parse().ok()?;
    let seconds: u8 = s.unwrap_or("0").parse().ok()?;

    if hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }

    Some((hours, minutes, seconds))
}

pub(super) fn normalize_reset_time_hms_lossy(input: &str) -> String {
    let Some((h, m, s)) = parse_reset_time_hms(input) else {
        return "00:00:00".to_string();
    };
    format!("{h:02}:{m:02}:{s:02}")
}

pub(super) fn normalize_reset_time_hms_strict(
    field: &str,
    input: &str,
) -> crate::shared::error::AppResult<String> {
    let Some((h, m, s)) = parse_reset_time_hms(input) else {
        return Err(format!("SEC_INVALID_INPUT: {field} must be HH:mm[:ss]").into());
    };
    Ok(format!("{h:02}:{m:02}:{s:02}"))
}

const MIN_STREAM_IDLE_TIMEOUT_SECONDS: u32 = 60;

pub(super) fn normalize_stream_idle_timeout_seconds(
    value: Option<u32>,
) -> crate::shared::error::AppResult<Option<u32>> {
    match value {
        None | Some(0) => Ok(None),
        Some(v) if v < MIN_STREAM_IDLE_TIMEOUT_SECONDS => Err(format!(
            "SEC_INVALID_INPUT: stream_idle_timeout_seconds must be 0 (disabled) or >= {MIN_STREAM_IDLE_TIMEOUT_SECONDS}, got {v}"
        )
        .into()),
        Some(v) if v <= MAX_STREAM_IDLE_TIMEOUT_SECONDS => Ok(Some(v)),
        Some(v) => Err(format!(
            "SEC_INVALID_INPUT: stream_idle_timeout_seconds must be within [0, {MAX_STREAM_IDLE_TIMEOUT_SECONDS}], got {v}"
        )
        .into()),
    }
}

pub(super) fn parse_positive_optional_u32(value: Option<i64>) -> Option<u32> {
    value
        .and_then(|raw| u32::try_from(raw).ok())
        .filter(|raw| *raw > 0)
}

pub(super) fn validate_limit_usd(
    field: &str,
    value: Option<f64>,
) -> crate::shared::error::AppResult<Option<f64>> {
    let Some(v) = value else {
        return Ok(None);
    };
    if !v.is_finite() {
        return Err(format!("SEC_INVALID_INPUT: {field} must be a finite number").into());
    }
    if v < 0.0 {
        return Err(format!("SEC_INVALID_INPUT: {field} must be >= 0").into());
    }
    if v > MAX_LIMIT_USD {
        return Err(format!("SEC_INVALID_INPUT: {field} must be <= {MAX_LIMIT_USD}").into());
    }
    Ok(Some(v))
}

pub(super) fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

pub(super) fn normalize_base_urls(
    base_urls: Vec<String>,
) -> crate::shared::error::AppResult<Vec<String>> {
    let initial_capacity = base_urls.len().clamp(1, MAX_PROVIDER_BASE_URLS);
    let mut out: Vec<String> = Vec::with_capacity(initial_capacity);
    let mut seen: HashSet<String> = HashSet::with_capacity(initial_capacity);

    for raw in base_urls {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        validate_max_chars("base_url", trimmed, MAX_PROVIDER_BASE_URL_CHARS)?;

        if !seen.insert(trimmed.to_string()) {
            continue;
        }

        if out.len() >= MAX_PROVIDER_BASE_URLS {
            return Err(format!(
                "SEC_INVALID_INPUT: base_urls must contain at most {MAX_PROVIDER_BASE_URLS} entries"
            )
            .into());
        }

        // Validate URL early to avoid runtime proxy errors.
        let parsed = reqwest::Url::parse(trimmed)
            .map_err(|e| format!("SEC_INVALID_INPUT: invalid base_url={trimmed}: {e}"))?;

        // Only http and https schemes are allowed.
        match parsed.scheme() {
            "http" | "https" => {}
            scheme => {
                return Err(format!(
                    "SEC_INVALID_INPUT: base_url must use http or https scheme, got '{scheme}': {trimmed}"
                )
                .into());
            }
        }

        out.push(trimmed.to_string());
    }

    if out.is_empty() {
        return Err("SEC_INVALID_INPUT: base_urls is required".into());
    }

    Ok(out)
}

pub(super) fn base_urls_from_row(base_url_fallback: &str, base_urls_json: &str) -> Vec<String> {
    let mut parsed: Vec<String> = serde_json::from_str::<Vec<String>>(base_urls_json)
        .ok()
        .unwrap_or_default()
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();

    // De-dup while preserving order.
    let mut seen: HashSet<String> = HashSet::with_capacity(parsed.len());
    parsed.retain(|v| seen.insert(v.clone()));

    if parsed.is_empty() {
        let fallback = base_url_fallback.trim();
        if fallback.is_empty() {
            return Vec::new();
        }
        return vec![fallback.to_string()];
    }

    parsed
}
