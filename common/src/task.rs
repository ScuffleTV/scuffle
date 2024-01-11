use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

thread_local! {
	static ABORT: RefCell<Arc<AtomicBool>> = RefCell::new(Arc::new(AtomicBool::new(false)));
	static ABORT_PANICED: Cell<bool> = Cell::new(false);
}

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
	#[error("task aborted")]
	Aborted,
	#[error("tokio error: {0}")]
	Tokio(#[from] tokio::task::JoinError),
	#[error("task panicked")]
	Panic,
}

struct TaskInner<J> {
	tag: String,
	abort: Arc<AtomicBool>,
	handle: Option<J>,
	drop_handle: fn(J, &TaskInner<J>),
	_unpin: std::marker::PhantomPinned,
}

impl<J> TaskInner<J> {
	fn new(tag: impl Display, abort: Arc<AtomicBool>, handle: J) -> Self {
		Self {
			tag: tag.to_string(),
			abort,
			handle: Some(handle),
			drop_handle: |_, _| {},
			_unpin: std::marker::PhantomPinned,
		}
	}

	fn abort(&self) {
		self.abort.store(true, std::sync::atomic::Ordering::Relaxed);
	}

	fn tag(&self) -> &str {
		&self.tag
	}

	fn set_drop_handle(&mut self, drop_handle: fn(J, &TaskInner<J>)) {
		self.drop_handle = drop_handle;
	}
}

impl<J> Drop for TaskInner<J> {
	fn drop(&mut self) {
		if let Some(handle) = self.handle.take() {
			(self.drop_handle)(handle, self);
		}
	}
}

pub struct Task<T>(TaskInner<std::thread::JoinHandle<Option<T>>>);

impl<T> Task<T> {
	pub fn new<F>(tag: impl Display, f: F) -> Self
	where
		T: Send + 'static,
		F: FnOnce() -> T + Send + 'static,
	{
		let abort = Arc::new(AtomicBool::new(false));
		let handle = std::thread::spawn(scope_thread(abort.clone(), f));
		Self(TaskInner::new(tag, abort, handle))
	}

	pub fn spawn<F>(tag: impl Display, f: F) -> Self
	where
		T: Send + 'static,
		F: FnOnce() -> T + Send + 'static,
	{
		Self::new(tag, f).with_drop_abort()
	}

	pub fn tag(&self) -> &str {
		self.0.tag()
	}

	pub fn with_drop_abort(mut self) -> Self {
		self.0.set_drop_handle(|_, inner| {
			inner.abort();
		});

		self
	}

	pub fn join(mut self) -> Result<T, TaskError> {
		self.0
			.handle
			.take()
			.unwrap()
			.join()
			.map_err(|_| TaskError::Panic)?
			.ok_or(TaskError::Aborted)
	}

	pub fn is_finished(&self) -> bool {
		self.0.handle.as_ref().unwrap().is_finished()
	}

	pub fn abort(&self) {
		self.0.abort();
	}
}

pub struct AsyncTask<T>(TaskInner<tokio::task::JoinHandle<Option<T>>>);

impl<T> AsyncTask<T> {
	pub fn new_blocking<F>(tag: impl Display, f: F) -> Self
	where
		T: Send + 'static,
		F: FnOnce() -> T + Send + 'static,
	{
		let abort = Arc::new(AtomicBool::new(false));
		let handle = tokio::task::spawn_blocking(scope_thread(abort.clone(), f));
		Self(TaskInner::new(tag, abort, handle))
	}

	pub fn new<F>(tag: impl Display, f: F) -> Self
	where
		T: Send + 'static,
		F: std::future::Future<Output = T> + Send + 'static,
	{
		let abort = Arc::new(AtomicBool::new(false));
		Self(TaskInner::new(tag, abort, tokio::task::spawn(async move { Some(f.await) })))
	}

	pub fn spawn<F>(tag: impl Display, f: F) -> Self
	where
		T: Send + 'static,
		F: std::future::Future<Output = T> + Send + 'static,
	{
		Self::new(tag, f).with_drop_abort()
	}

	pub fn spawn_blocking<F>(tag: impl Display, f: F) -> Self
	where
		T: Send + 'static,
		F: FnOnce() -> T + Send + 'static,
	{
		Self::new_blocking(tag, f).with_drop_abort()
	}

	pub fn with_drop_abort(mut self) -> Self {
		self.0.set_drop_handle(|handle, inner| {
			handle.abort();
			inner.abort();
		});
		self
	}

	pub fn tag(&self) -> &str {
		self.0.tag()
	}

	pub fn is_finished(&self) -> bool {
		self.0.handle.as_ref().unwrap().is_finished()
	}

	pub async fn join(&mut self) -> Result<T, TaskError> {
		match self.0.handle.as_mut().unwrap().await {
			Ok(Some(r)) => Ok(r),
			Err(err) if !err.is_cancelled() => Err(TaskError::Tokio(err)),
			_ => Err(TaskError::Aborted),
		}
	}

	pub fn abort(&self) {
		self.0.handle.as_ref().unwrap().abort();
		self.0.abort();
	}
}

fn scope_thread<F, T>(abort: Arc<AtomicBool>, f: F) -> impl FnOnce() -> Option<T>
where
	F: FnOnce() -> T,
{
	move || {
		set_abort(abort);

		// The reason this is allowed to be asserted is because we're catching the panic
		// and returning None instead of propagating it, if the panic was caused by a task abort.
		match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
			Ok(r) => Some(r),
			Err(err) => {
				if ABORT_PANICED.get() && err.is::<PanicAbort>() {
					None
				} else {
					std::panic::resume_unwind(err);
				}
			}
		}
	}
}

struct PanicAbort;
impl std::fmt::Display for PanicAbort {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("task aborted")
	}
}

pub fn check_abort() {
	// If the thread is panicking, we don't want to panic again in the panic handler
	if !std::thread::panicking() && !ABORT_PANICED.get() && is_aborted() {
		ABORT_PANICED.set(true);
		std::panic::resume_unwind(Box::new(PanicAbort));
	}
}

pub fn set_abort(abort: Arc<AtomicBool>) {
	ABORT.with(|a| {
		a.replace(abort);
	});
}

pub fn is_aborted() -> bool {
	ABORT.with(|abort| abort.borrow().load(std::sync::atomic::Ordering::Relaxed))
}

pub fn get_abort() -> Arc<AtomicBool> {
	ABORT.with(|abort| abort.borrow().clone())
}

pub struct AbortGuard;

impl AbortGuard {
	pub const fn new() -> Self {
		Self
	}
}

impl Drop for AbortGuard {
	fn drop(&mut self) {
		check_abort();
	}
}
