//! Usage: Run blocking work on Tauri async runtime with a stable label.

use crate::shared::error::{AppError, AppResult};
use std::sync::{Arc, OnceLock};
use tokio::sync::Semaphore;

const BLOCKING_MIN_CONCURRENT: usize = 8;
const BLOCKING_MAX_CONCURRENT: usize = 32;
const BLOCKING_PER_CORE: usize = 4;

static BLOCKING_LIMITER: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn blocking_concurrency_limit() -> usize {
    let parallelism = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(BLOCKING_MIN_CONCURRENT);
    blocking_concurrency_limit_for_parallelism(parallelism)
}

fn blocking_concurrency_limit_for_parallelism(parallelism: usize) -> usize {
    parallelism
        .saturating_mul(BLOCKING_PER_CORE)
        .clamp(BLOCKING_MIN_CONCURRENT, BLOCKING_MAX_CONCURRENT)
}

fn blocking_limiter() -> Arc<Semaphore> {
    BLOCKING_LIMITER
        .get_or_init(|| Arc::new(Semaphore::new(blocking_concurrency_limit())))
        .clone()
}

pub async fn run<T, E>(
    label: &'static str,
    f: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> AppResult<T>
where
    T: Send + 'static,
    E: Into<AppError> + Send + 'static,
{
    run_with_limiter(label, blocking_limiter(), f).await
}

async fn run_with_limiter<T, E>(
    label: &'static str,
    limiter: Arc<Semaphore>,
    f: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> AppResult<T>
where
    T: Send + 'static,
    E: Into<AppError> + Send + 'static,
{
    let permit = limiter.acquire_owned().await.map_err(|_| {
        AppError::new(
            "TASK_JOIN",
            format!("{label}: blocking task limiter closed"),
        )
    })?;

    let task = tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        f()
    });

    match task.await {
        Ok(result) => result.map_err(Into::into),
        Err(err) => {
            // Avoid forwarding JoinError display text to UI, because panic payloads may contain
            // user content (e.g., slicing errors include a snippet of the offending string).
            if let tauri::Error::JoinError(join_err) = err {
                if join_err.is_panic() {
                    tracing::error!(label, "blocking task panicked");
                    return Err(AppError::new(
                        "TASK_JOIN",
                        format!("{label}: task panicked"),
                    ));
                }

                tracing::warn!(label, "blocking task cancelled");
                return Err(AppError::new(
                    "TASK_JOIN",
                    format!("{label}: task cancelled"),
                ));
            }

            tracing::error!(label, "blocking task failed");
            Err(AppError::new("TASK_JOIN", format!("{label}: task failed")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn blocking_concurrency_limit_is_clamped_from_parallelism() {
        assert_eq!(
            blocking_concurrency_limit_for_parallelism(0),
            BLOCKING_MIN_CONCURRENT
        );
        assert_eq!(
            blocking_concurrency_limit_for_parallelism(1),
            BLOCKING_MIN_CONCURRENT
        );
        assert_eq!(blocking_concurrency_limit_for_parallelism(4), 16);
        assert_eq!(
            blocking_concurrency_limit_for_parallelism(99),
            BLOCKING_MAX_CONCURRENT
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_with_limiter_waits_before_spawning_when_full() {
        let limiter = Arc::new(Semaphore::new(1));
        let (first_started_tx, first_started_rx) = mpsc::channel();
        let (release_first_tx, release_first_rx) = mpsc::channel();
        let (second_started_tx, second_started_rx) = mpsc::channel();

        let first = tokio::spawn(run_with_limiter("first", limiter.clone(), move || {
            first_started_tx.send(()).expect("send first start");
            release_first_rx.recv().expect("release first");
            Ok::<_, AppError>(1)
        }));
        first_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("first task starts");

        let second = tokio::spawn(run_with_limiter("second", limiter, move || {
            second_started_tx.send(()).expect("send second start");
            Ok::<_, AppError>(2)
        }));
        assert!(second_started_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err());

        release_first_tx.send(()).expect("release first task");
        assert_eq!(first.await.expect("join first").expect("first result"), 1);
        second_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second task starts after permit is free");
        assert_eq!(
            second.await.expect("join second").expect("second result"),
            2
        );
    }
}
