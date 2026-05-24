//! Usage: Serialize gateway start/stop/rebind lifecycle transitions.

use std::sync::OnceLock;
use tokio::sync::{Mutex, MutexGuard};

static GATEWAY_LIFECYCLE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) type GatewayLifecycleGuard = MutexGuard<'static, ()>;

pub(crate) async fn lock() -> GatewayLifecycleGuard {
    GATEWAY_LIFECYCLE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await
}

#[cfg(test)]
mod tests {
    use super::lock;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::Duration;

    #[tokio::test]
    async fn lock_serializes_gateway_lifecycle_sections() {
        let first_guard = lock().await;
        let entered = Arc::new(AtomicBool::new(false));
        let entered_for_task = entered.clone();

        let task = tokio::spawn(async move {
            let _second_guard = lock().await;
            entered_for_task.store(true, Ordering::SeqCst);
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!entered.load(Ordering::SeqCst));

        drop(first_guard);

        tokio::time::timeout(Duration::from_millis(100), task)
            .await
            .expect("second lifecycle section should enter after first guard drops")
            .expect("task should not panic");
        assert!(entered.load(Ordering::SeqCst));
    }
}
