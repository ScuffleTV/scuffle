use std::sync::Arc;

use common::database::Ulid;
use pb::scuffle::video::v1::types::{event, Event};
use pb::scuffle::video::v1::EventsFetchRequest;

use crate::global::ApiGlobal;

pub async fn run<G: ApiGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	let mut event_stream = global
		.video_events_client()
		.clone()
		.fetch(EventsFetchRequest {
			target: pb::scuffle::video::v1::events_fetch_request::Target::Room.into(),
			max_delay_ms: 100,
			max_events: 100,
		})
		.await?
		.into_inner();

	while let Some(msg) = event_stream.message().await? {
		if let Some(Event {
			event: Some(event::Event::Room(event::Room {
				event: Some(evt),
				room_id: Some(room_id),
			})),
			timestamp,
			..
		}) = msg.event
		{
			match evt {
				event::room::Event::Ready(..) => {
					// TODO: index for channel_room_id
					sqlx::query("UPDATE users SET channel_live_viewer_count = 0, channel_live_viewer_count_updated_at = NOW(), channel_last_live_at = $1 WHERE channel_room_id = $2")
                        .bind(timestamp)
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
		}
	}

	Ok(())
}
