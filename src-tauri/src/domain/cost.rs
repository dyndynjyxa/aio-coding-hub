//! Usage: Calculate USD cost from usage metrics and model price JSON.

use serde_json::Value;
const CONTEXT_1M_TOKEN_THRESHOLD: i64 = 200_000;
const CONTEXT_1M_INPUT_PREMIUM_NUM: i128 = 2;
const CONTEXT_1M_INPUT_PREMIUM_DEN: i128 = 1;
const CONTEXT_1M_OUTPUT_PREMIUM_NUM: i128 = 3;
const CONTEXT_1M_OUTPUT_PREMIUM_DEN: i128 = 2; // 1.5

#[derive(Debug, Clone, Default)]
pub struct CostUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub cache_creation_5m_input_tokens: i64,
    pub cache_creation_1h_input_tokens: i64,
}

fn clamp_token_count(v: i64) -> i64 {
    v.max(0)
}

fn json_number_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.trim().to_string()),
        _ => None,
    }
}

fn parse_i64(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    s.parse::<i64>().ok()
}

fn pow10_i128(exp: u32) -> i128 {
    let mut v: i128 = 1;
    for _ in 0..exp {
        v = v.saturating_mul(10);
    }
    v
}

fn parse_decimal_to_femto(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (sign, rest) = if let Some(tail) = s.strip_prefix('-') {
        (-1i128, tail)
    } else if let Some(tail) = s.strip_prefix('+') {
        (1i128, tail)
    } else {
        (1i128, s)
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let (mantissa, exp10) = match rest.split_once(['e', 'E']) {
        Some((m, e)) => (m.trim(), parse_i64(e.trim())?),
        None => (rest, 0),
    };

    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((a, b)) => (a, b),
        None => (mantissa, ""),
    };

    let int_digits = int_part.trim();
    let frac_digits = frac_part.trim();

    let mut digits = String::with_capacity(int_digits.len() + frac_digits.len());
    digits.push_str(int_digits);
    digits.push_str(frac_digits);

    let digits = digits.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };

    let mantissa_int = digits.parse::<i128>().ok()?;
    let frac_places = frac_digits.len() as i64;

    // value = mantissa_int * 10^(exp10 - frac_places)
    // femto = value * 10^15 = mantissa_int * 10^(exp10 - frac_places + 15)
    let exp_femto = exp10 - frac_places + 15;
    let signed_mantissa = mantissa_int.saturating_mul(sign);

    let femto_i128 = if exp_femto >= 0 {
        let factor = pow10_i128(exp_femto as u32);
        signed_mantissa.saturating_mul(factor)
    } else {
        let div = pow10_i128((-exp_femto) as u32);
        if div == 0 {
            return None;
        }
        let q = signed_mantissa / div;
        let r = (signed_mantissa % div).abs();
        let half_up = r.saturating_mul(2) >= div.abs();
        if half_up {
            if signed_mantissa.is_negative() {
                q - 1
            } else {
                q + 1
            }
        } else {
            q
        }
    };

    if femto_i128 >= i64::MAX as i128 {
        return Some(i64::MAX);
    }
    if femto_i128 <= i64::MIN as i128 {
        return Some(i64::MIN);
    }

    Some(femto_i128 as i64)
}

fn get_femto_from_any(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(v) = get_femto(obj, key) {
            return Some(v);
        }
    }
    None
}

fn get_femto(obj: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    let value = obj.get(key)?;
    let s = json_number_to_string(value)?;
    parse_decimal_to_femto(&s)
}

fn mul_ratio_femto(value: i64, num: i128, den: i128) -> i64 {
    if den == 0 {
        return 0;
    }
    let v = value as i128;
    let n = v.saturating_mul(num);
    let q = n / den;
    let r = (n % den).abs();
    let half_up = r.saturating_mul(2) >= den.abs();
    let out = if half_up {
        if n.is_negative() {
            q - 1
        } else {
            q + 1
        }
    } else {
        q
    };

    if out >= i64::MAX as i128 {
        return i64::MAX;
    }
    if out <= i64::MIN as i128 {
        return i64::MIN;
    }
    out as i64
}

fn tiered_cost_with_separate_prices(tokens: i64, base: i64, premium: i64) -> i128 {
    if tokens <= 0 {
        return 0;
    }
    let base_tokens = tokens.min(CONTEXT_1M_TOKEN_THRESHOLD) as i128;
    let premium_tokens = tokens.saturating_sub(CONTEXT_1M_TOKEN_THRESHOLD).max(0) as i128;
    base_tokens.saturating_mul(base as i128) + premium_tokens.saturating_mul(premium as i128)
}

fn tiered_cost_with_multiplier(
    tokens: i64,
    base: i64,
    premium_num: i128,
    premium_den: i128,
) -> i128 {
    if tokens <= 0 {
        return 0;
    }
    let base_tokens = tokens.min(CONTEXT_1M_TOKEN_THRESHOLD) as i128;
    let premium_tokens = tokens.saturating_sub(CONTEXT_1M_TOKEN_THRESHOLD).max(0) as i128;
    let premium_cost = mul_ratio_femto(base, premium_num, premium_den) as i128;
    base_tokens.saturating_mul(base as i128) + premium_tokens.saturating_mul(premium_cost)
}

fn contains_context_1m(cli_key: &str, model: &str) -> bool {
    if cli_key != "claude" {
        return false;
    }
    model.to_ascii_lowercase().contains("1m")
}

fn multiplier_to_scaled_int(multiplier: f64) -> Option<i128> {
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return None;
    }
    let scaled = (multiplier * 1_000_000.0).round();
    if !scaled.is_finite() || scaled <= 0.0 {
        return None;
    }
    Some(scaled as i128)
}

fn apply_multiplier_femto(cost_femto: i128, multiplier: f64) -> Option<i128> {
    let scaled = multiplier_to_scaled_int(multiplier)?;
    let numerator = cost_femto.saturating_mul(scaled);
    let den: i128 = 1_000_000;
    let q = numerator / den;
    let r = (numerator % den).abs();
    let half_up = r.saturating_mul(2) >= den;
    let out = if half_up {
        if numerator.is_negative() {
            q - 1
        } else {
            q + 1
        }
    } else {
        q
    };
    Some(out)
}

fn finalize_i64(cost_femto: i128) -> Option<i64> {
    if cost_femto <= 0 {
        return None;
    }
    if cost_femto >= i64::MAX as i128 {
        return Some(i64::MAX);
    }
    Some(cost_femto as i64)
}

/// Options for cost calculation.
#[derive(Debug, Clone, Default)]
pub struct CostCalculationOptions {
    /// Whether priority service tier pricing should be applied (Codex fast mode).
    pub priority_service_tier_applied: bool,
}

#[cfg(test)]
pub fn calculate_cost_usd_femto(
    usage: &CostUsage,
    price_json: &str,
    multiplier: f64,
    cli_key: &str,
    model: &str,
) -> Option<i64> {
    calculate_cost_usd_femto_with_options(
        usage,
        price_json,
        multiplier,
        cli_key,
        model,
        &CostCalculationOptions::default(),
    )
}

pub fn calculate_cost_usd_femto_with_options(
    usage: &CostUsage,
    price_json: &str,
    multiplier: f64,
    cli_key: &str,
    model: &str,
    options: &CostCalculationOptions,
) -> Option<i64> {
    let parsed: Value = serde_json::from_str(price_json).ok()?;
    let obj = parsed.as_object()?;

    // If priority service tier is applied, prefer priority pricing fields
    let input_cost = if options.priority_service_tier_applied {
        get_femto_from_any(
            obj,
            &[
                "input_cost_per_token_priority",
                "input_cost_per_token",
                "input_cost_per_cached_token",
            ],
        )
    } else {
        get_femto_from_any(
            obj,
            &["input_cost_per_token", "input_cost_per_cached_token"],
        )
    }
    .unwrap_or(0);

    let output_cost = if options.priority_service_tier_applied {
        get_femto_from_any(
            obj,
            &[
                "output_cost_per_token_priority",
                "output_cost_per_token",
                "output_cost_per_cached_token",
            ],
        )
    } else {
        get_femto_from_any(
            obj,
            &["output_cost_per_token", "output_cost_per_cached_token"],
        )
    }
    .unwrap_or(0);

    let input_cost_above_200k = get_femto(obj, "input_cost_per_token_above_200k_tokens");
    let output_cost_above_200k = get_femto(obj, "output_cost_per_token_above_200k_tokens");

    let cache_creation_5m_cost = get_femto(obj, "cache_creation_input_token_cost")
        .or_else(|| {
            if input_cost > 0 {
                Some(mul_ratio_femto(input_cost, 5, 4))
            } else {
                None
            }
        })
        .unwrap_or(0);

    let cache_creation_1h_cost = get_femto(obj, "cache_creation_input_token_cost_above_1hr")
        .or_else(|| {
            if input_cost > 0 {
                Some(mul_ratio_femto(input_cost, 2, 1))
            } else {
                None
            }
        })
        .or((cache_creation_5m_cost > 0).then_some(cache_creation_5m_cost))
        .unwrap_or(0);

    let cache_read_cost = get_femto(obj, "cache_read_input_token_cost")
        .or_else(|| {
            if input_cost > 0 {
                Some(mul_ratio_femto(input_cost, 1, 10))
            } else {
                None
            }
        })
        .or_else(|| {
            if output_cost > 0 {
                Some(mul_ratio_femto(output_cost, 1, 10))
            } else {
                None
            }
        })
        .unwrap_or(0);

    let input_tokens = clamp_token_count(usage.input_tokens);
    let output_tokens = clamp_token_count(usage.output_tokens);
    let cache_read_input_tokens = clamp_token_count(usage.cache_read_input_tokens);
    let cache_creation_5m_input_tokens = clamp_token_count(usage.cache_creation_5m_input_tokens);
    let cache_creation_1h_input_tokens = clamp_token_count(usage.cache_creation_1h_input_tokens);
    let cache_creation_input_tokens = clamp_token_count(usage.cache_creation_input_tokens);
    let priced_cache_creation_input_tokens =
        if cache_creation_5m_input_tokens > 0 || cache_creation_1h_input_tokens > 0 {
            cache_creation_5m_input_tokens.saturating_add(cache_creation_1h_input_tokens)
        } else {
            cache_creation_input_tokens
        };

    // OpenAI reports cache reads and writes as subsets of total input. Gemini only reports cache
    // reads as a subset, while Claude bills cache buckets in addition to its input count.
    let billable_input_tokens = match cli_key {
        "codex" | "grok" => input_tokens
            .saturating_sub(cache_read_input_tokens)
            .saturating_sub(priced_cache_creation_input_tokens),
        "gemini" => input_tokens.saturating_sub(cache_read_input_tokens),
        _ => input_tokens,
    };

    let context_1m_applied = contains_context_1m(cli_key, model);

    let mut cost_femto: i128 = 0;

    if billable_input_tokens > 0 && input_cost > 0 {
        cost_femto += if context_1m_applied {
            tiered_cost_with_multiplier(
                billable_input_tokens,
                input_cost,
                CONTEXT_1M_INPUT_PREMIUM_NUM,
                CONTEXT_1M_INPUT_PREMIUM_DEN,
            )
        } else if let Some(premium) = input_cost_above_200k {
            tiered_cost_with_separate_prices(billable_input_tokens, input_cost, premium)
        } else {
            (billable_input_tokens as i128).saturating_mul(input_cost as i128)
        };
    }

    if output_tokens > 0 && output_cost > 0 {
        cost_femto += if context_1m_applied {
            tiered_cost_with_multiplier(
                output_tokens,
                output_cost,
                CONTEXT_1M_OUTPUT_PREMIUM_NUM,
                CONTEXT_1M_OUTPUT_PREMIUM_DEN,
            )
        } else if let Some(premium) = output_cost_above_200k {
            tiered_cost_with_separate_prices(output_tokens, output_cost, premium)
        } else {
            (output_tokens as i128).saturating_mul(output_cost as i128)
        };
    }

    if cache_read_input_tokens > 0 && cache_read_cost > 0 {
        cost_femto += (cache_read_input_tokens as i128).saturating_mul(cache_read_cost as i128);
    }

    // Prefer TTL-specific breakdown; else fall back to total tokens as 5m cost.
    if (cache_creation_5m_input_tokens > 0 || cache_creation_1h_input_tokens > 0)
        && (cache_creation_5m_cost > 0 || cache_creation_1h_cost > 0)
    {
        let part_5m = if cache_creation_5m_input_tokens > 0 && cache_creation_5m_cost > 0 {
            if context_1m_applied {
                tiered_cost_with_multiplier(
                    cache_creation_5m_input_tokens,
                    cache_creation_5m_cost,
                    CONTEXT_1M_INPUT_PREMIUM_NUM,
                    CONTEXT_1M_INPUT_PREMIUM_DEN,
                )
            } else {
                (cache_creation_5m_input_tokens as i128)
                    .saturating_mul(cache_creation_5m_cost as i128)
            }
        } else {
            0
        };

        let part_1h = if cache_creation_1h_input_tokens > 0 && cache_creation_1h_cost > 0 {
            if context_1m_applied {
                tiered_cost_with_multiplier(
                    cache_creation_1h_input_tokens,
                    cache_creation_1h_cost,
                    CONTEXT_1M_INPUT_PREMIUM_NUM,
                    CONTEXT_1M_INPUT_PREMIUM_DEN,
                )
            } else {
                (cache_creation_1h_input_tokens as i128)
                    .saturating_mul(cache_creation_1h_cost as i128)
            }
        } else {
            0
        };

        cost_femto += part_5m.saturating_add(part_1h);
    } else if cache_creation_input_tokens > 0 && cache_creation_5m_cost > 0 {
        cost_femto += if context_1m_applied {
            tiered_cost_with_multiplier(
                cache_creation_input_tokens,
                cache_creation_5m_cost,
                CONTEXT_1M_INPUT_PREMIUM_NUM,
                CONTEXT_1M_INPUT_PREMIUM_DEN,
            )
        } else {
            (cache_creation_input_tokens as i128).saturating_mul(cache_creation_5m_cost as i128)
        };
    }
    let cost_femto = apply_multiplier_femto(cost_femto, multiplier)?;
    finalize_i64(cost_femto)
}

#[cfg(test)]
mod tests;
