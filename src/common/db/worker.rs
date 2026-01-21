use tokio::{
    sync::mpsc::{self, Receiver},
    time::{timeout, Duration},
};

use crate::{
    common::db::{DbPool, SqlxJob, SqlxJobOrFlush, SqlxJobResult, SqlxJobSender, SqlxTransaction},
    config::CONFIG,
};

pub fn start_sqlx_worker(db_pool: DbPool) -> SqlxJobSender {
    let (tx, mut rx) = mpsc::channel::<SqlxJobOrFlush>(CONFIG.db_query_batch_size);

    tokio::spawn(async move {
        tracing::info!("Spawned SQLx worker.");
        let mut batch: Vec<SqlxJob> = Vec::with_capacity(CONFIG.db_query_batch_size);
        sqlx_worker_loop(db_pool, &mut batch, &mut rx).await;
    });

    tx
}

async fn sqlx_worker_loop(
    db_pool: DbPool,
    batch: &mut Vec<SqlxJob>,
    rx: &mut Receiver<SqlxJobOrFlush>,
) {
    // loop and wait to rx.recv() a job
    // when a job is received, add it to the batch and start a timeout
    // if the timeout expires, flush the batch
    // if a flush command is received, flush the batch
    loop {
        match timeout(
            Duration::from_millis(CONFIG.db_query_batch_timeout_ms),
            rx.recv(),
        )
        .await
        {
            Ok(Some(SqlxJobOrFlush::Job(job))) => {
                batch.push(job);
                if batch.len() < CONFIG.db_query_batch_size {
                    continue;
                }
            }
            Ok(Some(SqlxJobOrFlush::Flush)) | Err(_) => {
                if batch.is_empty() {
                    continue;
                }
            }
            Ok(None) => break,
        }
        let batch_len = batch.len();

        tracing::debug!("Batched {batch_len} queries. Beginning transaction.");
        let mut db_tx: SqlxTransaction = match db_pool.begin().await {
            Ok(tx_db) => tx_db,
            Err(e) => {
                tracing::error!("Failed to start transaction: {e}");
                batch.clear();
                continue;
            }
        };
        for job in batch.drain(..) {
            let res: SqlxJobResult = job.query.execute(&mut *db_tx).await;
            let _ = job.result_tx.send(res);
        }
        if let Err(e) = db_tx.commit().await {
            tracing::error!("Failed to commit transaction: {e}");
        }
        tracing::debug!("Committed transaction of {batch_len} queries.")
    }
}
