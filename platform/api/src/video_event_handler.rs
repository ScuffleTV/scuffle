use std::sync::Arc;

use anyhow::Context;
use common::database::Ulid;
use pb::scuffle::video::v1::types::{event, Event};
use pb::scuffle::video::v1::{EventsAckRequest, EventsFetchRequest};

use crate::global::ApiGlobal;

pub async fn run<G: ApiGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	loop {
		let mut event_stream = global
			.video_events_client()
			.clone()
			.fetch(EventsFetchRequest {
				target: pb::scuffle::video::v1::events_fetch_request::Target::Room.into(),
				max_delay_ms: 1000,
				max_events: 100,
			})
			.await
			.context("failed to fetch events")?
			.into_inner();

		while let Some(msg) = event_stream.message().await? {
			if let Some(Event {
				event: Some(event::Event::Room(event)),
				timestamp,
				event_id: Some(evt_id),
			}) = msg.event
			{
				let action = match handle_room_event(&global, event, timestamp).await {
					Ok(_) => pb::scuffle::video::v1::events_ack_request::Action::Ack(true),
					Err(err) => {
						tracing::warn!(err = %err, "failed to handle event, requeueing");
						pb::scuffle::video::v1::events_ack_request::Action::RequeueDelayMs(5000)
					}
				};
				global
					.video_events_client()
					.clone()
					.ack(EventsAckRequest {
						id: Some(evt_id),
						action: Some(action),
					})
					.await?;
			}
		}
	}
}

async fn handle_room_event<G: ApiGlobal>(global: &Arc<G>, event: event::Room, timestamp: i64) -> anyhow::Result<()> {
	let room_id = event.room_id.as_ref().unwrap();
	match event.event.unwrap() {
		event::room::Event::Ready(..) => {
			// TODO: index for channel_room_id
			sqlx::query("UPDATE users SET channel_live_viewer_count = 0, channel_live_viewer_count_updated_at = NOW(), channel_last_live_at = $1 WHERE channel_room_id = $2")
				.bind(chrono::NaiveDateTime::from_timestamp_millis(timestamp))
				.bind(Ulid::from(room_id.into_ulid()))
				.execute(global.db().as_ref())
				.await?;
		}
		event::room::Event::Disconnected(..) => {
			sqlx::query(
				"UPDATE users SET channel_live_viewer_count = NULL, channel_live_viewer_count_updated_at = NOW() WHERE channel_room_id = $1",
			)
			.bind(Ulid::from(room_id.into_ulid()))
			.execute(global.db().as_ref())
			.await?;
		}
		_ => {}
	}
	Ok(())
}
