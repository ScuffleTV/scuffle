use std::sync::Arc;

use scuffle_image_processor_proto::{event_callback, EventCallback, EventQueue as EventTopic, OutputFile};

use crate::database::Job;
use crate::event_queue::EventQueue;
use crate::global::Global;
use crate::worker::JobError;

#[tracing::instrument(skip(global, job, event_topic), fields(topic = %event_topic.topic, name = %event_topic.name, job_id = %job.id))]
pub async fn on_event(global: &Arc<Global>, job: &Job, event_topic: &EventTopic, event: event_callback::Event) {
	let Some(queue) = global.event_queue(&event_topic.name) else {
		tracing::warn!("event queue not found: {}", event_topic.name);
		return;
	};

	if let Err(err) = queue
		.publish(
			&event_topic.topic,
			EventCallback {
				id: job.id.to_string(),
				timestamp: chrono::Utc::now().timestamp() as u64,
				event: Some(event),
			},
		)
		.await
	{
		tracing::error!("failed to publish event: {err}");
	}
}

fn start_event(_: &Job) -> event_callback::Event {
	event_callback::Event::Start(event_callback::Start {})
}

fn success_event(_: &Job, drive: String, files: Vec<OutputFile>) -> event_callback::Event {
	event_callback::Event::Success(event_callback::Success { drive, files })
}

fn fail_event(_: &Job, err: JobError) -> event_callback::Event {
	event_callback::Event::Fail(event_callback::Fail { error: Some(err.into()) })
}

fn cancel_event(_: &Job) -> event_callback::Event {
	event_callback::Event::Cancel(event_callback::Cancel {})
}

pub async fn on_start(global: &Arc<Global>, job: &Job) {
	if let Some(on_start) = &job.task.events.as_ref().and_then(|events| events.on_start.as_ref()) {
		on_event(global, job, on_start, start_event(job)).await;
	}
}

pub async fn on_success(global: &Arc<Global>, job: &Job, drive: String, files: Vec<OutputFile>) {
	if let Some(on_success) = &job.task.events.as_ref().and_then(|events| events.on_success.as_ref()) {
		on_event(global, job, on_success, success_event(job, drive, files)).await;
	}
}

pub async fn on_failure(global: &Arc<Global>, job: &Job, err: JobError) {
	if let Some(on_failure) = &job.task.events.as_ref().and_then(|events| events.on_failure.as_ref()) {
		on_event(global, job, on_failure, fail_event(job, err)).await;
	}
}

pub async fn on_cancel(global: &Arc<Global>, job: &Job) {
	if let Some(on_cancel) = &job.task.events.as_ref().and_then(|events| events.on_cancel.as_ref()) {
		on_event(global, job, on_cancel, cancel_event(job)).await;
	}
}
