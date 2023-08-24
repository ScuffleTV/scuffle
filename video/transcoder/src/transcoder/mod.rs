use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, bail, Result};
use async_nats::jetstream::consumer::pull::Config;
use futures::StreamExt;
use tokio::select;
use tokio_util::sync::CancellationToken;

use crate::{global::GlobalState, transcoder::job::handle_message};

pub(crate) mod job;

pub async fn run(global: Arc<GlobalState>) -> Result<()> {
    let stream = global
        .jetstream
        .get_stream(global.config.transcoder.transcoder_request_subject.clone())
        .await?;

    let consumer = stream
        .create_consumer(Config {
            name: Some("transcoder".to_string()),
            durable_name: Some("transcoder".to_string()),
            ..Default::default()
        })
        .await?;

    let mut messages = consumer.messages().await?;

    let shutdown_token = CancellationToken::new();
    let child_token = shutdown_token.child_token();
    let _drop_guard = shutdown_token.clone().drop_guard();

    loop {
        select! {
            m = messages.next() => {
                let Some(m) = m else {
                    bail!("nats stream closed");
                };

                let m = m.map_err(|e| {
                    anyhow!("failed to get message: {}", e)
                })?;

                tokio::spawn(handle_message(global.clone(), m, child_token.clone()));
            },
            _ = global.ctx.done() => {
                tracing::debug!("context done");
                break;
            }
        }
    }

    drop(messages);
    drop(consumer);

    tokio::time::sleep(Duration::from_millis(100)).await;

    global.nats.flush().await?;

    Ok(())
}
