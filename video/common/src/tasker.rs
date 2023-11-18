use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use futures_util::future::BoxFuture;
use futures_util::Future;

pub type TaskFuture<O, E> = BoxFuture<'static, Result<O, E>>;

pub type TaskGenerator<T, O, E> = Arc<dyn Fn(T) -> TaskFuture<O, E> + Send + Sync>;

#[derive(Clone)]
#[must_use = "tasks do nothing unless queued"]
pub struct Task<T, D, O, E> {
	id: String,
	generator: TaskGenerator<T, O, E>,
	retry_count: u32,
	domain: D,
}

impl<T, D, O, E> std::fmt::Debug for Task<T, D, O, E> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Task")
			.field("id", &self.id)
			.field("retry_count", &self.retry_count)
			.finish()
	}
}

impl<T, D, O: 'static, E: 'static> Task<T, D, O, E> {
	pub fn new(id: String, generator: TaskGenerator<T, O, E>, domain: D) -> Self {
		Self {
			id,
			generator,
			retry_count: 0,
			domain,
		}
	}

	pub fn id(&self) -> &str {
		&self.id
	}

	pub fn domain(&self) -> &D {
		&self.domain
	}

	pub fn retry(&mut self) {
		self.retry_count += 1;
	}

	pub fn retry_count(&self) -> u32 {
		self.retry_count
	}

	fn run(&self, state: T) -> TaskFuture<O, E> {
		let retry = self.retry_count;
		let fut = (self.generator)(state);

		Box::pin(async move {
			if retry > 0 {
				tokio::time::sleep(std::time::Duration::from_millis(retry as u64 * 100)).await;
			}
			fut.await
		})
	}
}

struct ActiveTask<T, D, O, E> {
	task: Task<T, D, O, E>,
	future: TaskFuture<O, E>,
}

struct Tasker<T, D, O, E> {
	tasks: VecDeque<Task<T, D, O, E>>,
	active_task: Option<ActiveTask<T, D, O, E>>,
}

pub struct MultiTasker<T: Clone, D: Ord, O, E> {
	taskers: BTreeMap<D, Tasker<T, D, O, E>>,
}

impl<T: Clone, D: Ord, O: 'static, E: 'static> Default for MultiTasker<T, D, O, E> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: Clone, D: Ord, O: 'static, E: 'static> MultiTasker<T, D, O, E> {
	pub fn new() -> Self {
		Self {
			taskers: BTreeMap::new(),
		}
	}

	pub fn add_domain(&mut self, domain: D) {
		self.add_tasker(domain, Tasker::new());
	}

	fn add_tasker(&mut self, domain: D, tasker: Tasker<T, D, O, E>) {
		self.taskers.insert(domain, tasker);
	}

	pub fn requeue(&mut self, mut task: Task<T, D, O, E>) -> Option<()> {
		task.retry();
		self.taskers.get_mut(task.domain())?.requeue(task);
		Some(())
	}

	pub fn abort(&mut self, domain: &D, id: &str) {
		if let Some(tasker) = self.taskers.get_mut(domain) {
			tasker.abort(id);
		}
	}

	pub fn submit(
		&mut self,
		domain: D,
		id: impl AsRef<str>,
		f: impl Fn(T) -> BoxFuture<'static, Result<O, E>> + Send + Sync + 'static,
	) -> Option<()> {
		self.submit_task(Task::new(id.as_ref().to_owned(), Arc::new(f), domain))
	}

	pub fn submit_task(&mut self, task: Task<T, D, O, E>) -> Option<()> {
		self.taskers.get_mut(task.domain())?.submit_task(task);
		Some(())
	}

	pub fn submit_with_abort(
		&mut self,
		domain: D,
		id: impl AsRef<str>,
		f: impl Fn(T) -> BoxFuture<'static, Result<O, E>> + Send + Sync + 'static,
	) {
		self.abort(&domain, id.as_ref());
		self.submit(domain, id, f);
	}

	pub fn submit_task_with_abort(&mut self, task: Task<T, D, O, E>) -> Option<()> {
		self.abort(task.domain(), task.id().as_ref());
		self.submit_task(task)
	}

	pub async fn next_task(&mut self, state: T) -> Option<Result<Task<T, D, O, E>, (Task<T, D, O, E>, E)>> {
		let futures = self
			.taskers
			.values_mut()
			.filter_map(|tasker| tasker.next_task(state.clone()))
			.map(Box::pin)
			.collect::<Vec<_>>();
		if futures.is_empty() {
			return None;
		}

		let (r, _, _) = futures::future::select_all(futures).await;
		Some(r)
	}
}

impl<T, D, O: 'static, E: 'static> Tasker<T, D, O, E> {
	pub fn new() -> Self {
		Self {
			tasks: VecDeque::new(),
			active_task: None,
		}
	}

	pub fn requeue(&mut self, mut task: Task<T, D, O, E>) {
		task.retry();
		self.tasks.push_front(task);
	}

	pub fn abort(&mut self, id: &str) {
		self.tasks.retain(|t| t.id() != id);
	}

	pub fn submit_task(&mut self, task: Task<T, D, O, E>) {
		self.tasks.push_back(task);
	}

	pub fn next_task(
		&mut self,
		state: T,
	) -> Option<impl Future<Output = Result<Task<T, D, O, E>, (Task<T, D, O, E>, E)>> + '_> {
		if self.active_task.is_none() {
			let task = self.tasks.pop_front()?;
			let future = task.run(state);
			self.active_task = Some(ActiveTask { task, future });
		}

		Some(async {
			let active_task = self.active_task.as_mut().unwrap();
			let result = active_task.future.as_mut().await;
			let active_task = self.active_task.take().unwrap();

			if let Err(e) = result {
				Err((active_task.task, e))
			} else {
				Ok(active_task.task)
			}
		})
	}
}
