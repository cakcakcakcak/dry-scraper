use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use crate::config::CONFIG;
use crate::db::{DbPool, SqlxJob, SqlxJobSender};

pub fn start_sqlx_worker(pool: DbPool) -> SqlxJobSender {
    let (tx, mut rx) = mpsc::channel::<SqlxJob>(CONFIG.db_query_batch_size);

    tokio::spawn(async move {
        tracing::info!("Spawned SQLx worker.");
        let mut batch: Vec<SqlxJob> = Vec::with_capacity(CONFIG.db_query_batch_size);

        loop {
            match timeout(
                Duration::from_millis(CONFIG.db_query_batch_timeout_ms),
                rx.recv(),
            )
            .await
            {
                Ok(Some(job)) => batch.push(job),
                Ok(None) => break,
                Err(_) => {}
            }

            let batch_len: usize = batch.len();
            if batch.len() >= CONFIG.db_query_batch_size || (!batch.is_empty() && rx.is_empty()) {
                tracing::info!("Batched {batch_len} queries. Beginning transaction.");
                let tx_db: sqlx::Transaction<'static, sqlx::Postgres> = match pool.begin().await {
                    Ok(tx_db) => tx_db,
                    Err(e) => {
                        tracing::error!("Failed to start transaction: {e}");
                        batch.clear();
                        continue;
                    }
                };
                for (job, result_tx) in batch.drain(..) {
                    let res = job.await;
                    let _ = result_tx.send(res);
                }
                if let Err(e) = tx_db.commit().await {
                    tracing::error!("Failed to commit transaction: {e}");
                }
                tracing::info!("Committed transaction of {batch_len} queries.")
            }
        }
    });

    tx
}
