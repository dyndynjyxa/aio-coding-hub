//! Usage: Network probe helpers (HTTP HEAD/GET latency measurement).

use std::time::{Duration, Instant};

pub(crate) async fn probe_base_url_ms(
    client: &reqwest::Client,
    base_url: &str,
    timeout: Duration,
) -> Result<u64, String> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err("SEC_INVALID_INPUT: base_url is required".to_string());
    }

    let url = reqwest::Url::parse(base_url)
        .map_err(|e| format!("SEC_INVALID_INPUT: invalid base_url={base_url}: {e}"))?;

    let started = Instant::now();

    let head_result = client.head(url.clone()).timeout(timeout).send().await;
    if head_result.is_ok() {
        return Ok(started.elapsed().as_millis() as u64);
    }

    client
        .get(url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| format!("PING_ERROR: {e}"))?;

    Ok(started.elapsed().as_millis() as u64)
}
