use std::{collections::VecDeque, sync::Arc};

use bytes::Bytes;
use futures_util::future::BoxFuture;

use crate::global::GlobalState;

pub type TaskFuture = BoxFuture<'static, Result<(), TaskError>>;

type TaskGenerator = Arc<dyn Fn(&str, Arc<GlobalState>) -> TaskFuture + Send + Sync>;

#[derive(Clone)]
pub enum TaskJob {
    UploadMetadata(Bytes),
    UploadMedia(Bytes),
    DeleteMedia,
    Custom(TaskGenerator),
}

impl std::fmt::Debug for TaskJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskJob::UploadMetadata(_) => write!(f, "UploadMetadata"),
            TaskJob::UploadMedia(_) => write!(f, "UploadMedia"),
            TaskJob::DeleteMedia => write!(f, "DeleteMedia"),
            TaskJob::Custom(_) => write!(f, "Custom"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    job: TaskJob,
    key: String,
    retry_count: u32,
}

impl Task {
    pub fn new(key: String, job: TaskJob) -> Self {
        Self {
            job,
            key,
            retry_count: 0,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    fn run(&self, global: &Arc<GlobalState>) -> TaskFuture {
        let global = global.clone();

        match &self.job {
            TaskJob::UploadMetadata(data) => {
                let key = self.key.clone();
                let data = data.clone();
                Box::pin(async move {
                    global.metadata_store
                        .put(&key, data)
                        .await?;
                    Ok(())
                })
            }
            TaskJob::UploadMedia(data) => {
                let key = self.key.clone();
                let data = data.clone();
                Box::pin(async move {
                    let mut cursor = std::io::Cursor::new(data);

                    global.media_store
                        .put(key.as_str(), &mut cursor)
                        .await?;
                    Ok(())
                })
            }
            TaskJob::DeleteMedia => {
                let key = self.key.clone();
                Box::pin(async move {
                    global.media_store
                        .delete(&key)
                        .await?;
                    Ok(())
                })
            }
            TaskJob::Custom(f) => f(&self.key, global),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("failed to upload metadata: {0}")]
    UploadMetadata(#[from] async_nats::jetstream::kv::PutError),
    #[error("failed to upload media: {0}")]
    UploadMedia(#[from] async_nats::jetstream::object_store::PutError),
    #[error("failed to delete metadata: {0}")]
    DeleteMetadata(#[from] async_nats::jetstream::kv::UpdateError),
    #[error("failed to delete media: {0}")]
    DeleteMedia(#[from] async_nats::jetstream::object_store::DeleteError),
    #[error("custom task failed: {0}")]
    Custom(#[from] anyhow::Error),
}

struct ActiveTask {
    task: Task,
    future: TaskFuture,
}

pub struct Tasker {
    tasks: VecDeque<Task>,
    active_task: Option<ActiveTask>,
}

impl Tasker {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            active_task: None,
        }
    }

    pub fn requeue(&mut self, mut task: Task) {
        task.retry();
        self.tasks.push_front(task);
    }

    pub fn custom(&mut self, key: String, f: impl Fn(&str, Arc<GlobalState>) -> BoxFuture<'static, Result<(), TaskError>> + Send + Sync + 'static) {
        self.abort_task(&key);
        self.tasks.push_back(Task::new(key, TaskJob::Custom(Arc::new(f))));
    }

    pub fn upload_metadata(&mut self, key: String, data: Bytes) {
        self.abort_task(&key);
        self.tasks.push_back(Task::new(key, TaskJob::UploadMetadata(data)));
    }

    pub fn upload_media(&mut self, key: String, data: Bytes) {
        self.abort_task(&key);
        self.tasks.push_back(Task::new(key, TaskJob::UploadMedia(data)));
    }

    pub fn delete_media(&mut self, key: String) {
        self.abort_task(&key);
        self.tasks.push_back(Task::new(key, TaskJob::DeleteMedia));
    }

    pub fn abort_task(&mut self, key: &str) {
        self.tasks.retain(|task| task.key() != key);
    }

    pub async fn next_task(&mut self, global: &Arc<GlobalState>) -> Option<Result<Task, (Task, TaskError)>> {
        if self.active_task.is_none() {
            let task = self.tasks.pop_front()?;
            let future = task.run(global);
            self.active_task = Some(ActiveTask { task, future });
        }

        let active_task = self.active_task.as_mut().unwrap();
        let result = active_task.future.as_mut().await;
        let active_task = self.active_task.take().unwrap();

        if let Err(e) = result {
            Some(Err((active_task.task, e)))
        } else {
            Some(Ok(active_task.task))            
        }
    }
}
