//! Best-effort dashboard statistics, deliberately off the block-response path.
//! No polling, timers, per-request tasks, or shutdown persistence guarantees.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use tokio::sync::Notify;

use crate::db::EngineDB;

#[derive(Default)]
pub(super) struct BlockedAttemptRecorder {
    pending: AtomicU64,
    notification: Notify,
}

impl BlockedAttemptRecorder {
    /// Never wait for SQLite before denying a request.
    pub fn record(&self) {
        // Wake the single worker only when an empty batch becomes nonempty.
        // Notify retains a permit even if the worker is currently writing.
        if self.pending.fetch_add(1, Ordering::Relaxed) == 0 {
            self.notification.notify_one();
        }
    }

    async fn next_batch(&self) -> u64 {
        loop {
            self.notification.notified().await;
            let count = self.pending.swap(0, Ordering::Relaxed);
            if count != 0 {
                return count;
            }
        }
    }

    /// Run once per proxy lifetime. Lost statistics on DB failure or shutdown
    /// are acceptable; enforcement must never depend on this worker.
    pub async fn persist(self: Arc<Self>, db: Arc<EngineDB>) {
        loop {
            let count = self.next_batch().await;
            let db = Arc::clone(&db);
            let result = tokio::task::spawn_blocking(move || {
                // The daily dashboard column is an i32. Losing excess events
                // is preferable to overflowing the aggregate.
                let amount = i32::try_from(count).unwrap_or(i32::MAX);
                db.increment_blocked_attempts_by(chrono::Utc::now().date_naive(), amount)
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%error, "could not persist blocked-attempt statistics");
                }
                Err(error) => {
                    tracing::warn!(%error, "blocked-attempt statistics worker failed");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn records_are_aggregated_without_a_periodic_timer() {
        let recorder = BlockedAttemptRecorder::default();
        for _ in 0..20 {
            recorder.record();
        }

        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), recorder.next_batch())
                .await
                .unwrap(),
            20
        );
        assert_eq!(recorder.pending.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn a_new_burst_while_worker_is_busy_is_not_lost() {
        let recorder = BlockedAttemptRecorder::default();

        recorder.record();
        assert_eq!(recorder.next_batch().await, 1);

        // Simulates events arriving while the previous batch is being
        // written, before the worker begins waiting again.
        for _ in 0..5 {
            recorder.record();
        }
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), recorder.next_batch())
                .await
                .unwrap(),
            5
        );
    }

    #[tokio::test]
    async fn concurrent_recorders_do_not_lose_increments() {
        let recorder = Arc::new(BlockedAttemptRecorder::default());
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let recorder = Arc::clone(&recorder);
            tasks.push(tokio::spawn(async move {
                for _ in 0..100 {
                    recorder.record();
                }
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }

        assert_eq!(recorder.next_batch().await, 800);
    }
}
