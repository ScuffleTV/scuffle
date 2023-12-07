use std::sync::Arc;

use pb::scuffle::video::v1::events_fetch_request::Target;
use pb::scuffle::video::v1::types::{event, Event};
use prost::Message;
use video_common::keys::event_subject;

use crate::global::ApiGlobal;

pub async fn emit<G: ApiGlobal>(global: &Arc<G>, org_id: ulid::Ulid, target: Target, event: event::Event) {
	global
		.jetstream()
		.publish(
			event_subject(org_id, target),
			Event {
				timestamp: chrono::Utc::now().timestamp_millis(),
				event_id: Some(ulid::Ulid::new().into()),
				event: Some(event),
			}
			.encode_to_vec()
			.into(),
		)
		.await
		.map_err(|e| {
			tracing::error!(err = %e, "failed to publish event");
		})
		.ok();
}
