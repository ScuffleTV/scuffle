use std::{cell::RefCell, future::Future, sync::Arc};

use rand::Rng;
use tokio::task::JoinHandle;

pub enum Runtime {
    Steal(tokio::runtime::Runtime),
    NoSteal(Arc<NoStealRuntime>),
}

impl Runtime {
    pub fn new_steal(thread_count: usize, name: &str) -> std::io::Result<Self> {
        Ok(Self::Steal(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(thread_count)
                .thread_name(name)
                .enable_all()
                .build()?,
        ))
    }

    pub fn new_no_steal(thread_count: usize, name: &str) -> std::io::Result<Self> {
        Ok(Self::NoSteal(NoStealRuntime::new(thread_count, name)?))
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            Self::Steal(runtime) => runtime.spawn(future),
            Self::NoSteal(runtime) => runtime.spawn(future),
        }
    }

    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        match self {
            Self::Steal(runtime) => runtime.block_on(future),
            Self::NoSteal(runtime) => runtime.block_on(future),
        }
    }
}

pub struct NoStealRuntime {
    runtimes: Vec<tokio::runtime::Runtime>,
}

struct NoStealRuntimeThreadData {
    runtime: Arc<NoStealRuntime>,
    idx: usize,
}

thread_local! {
    static NO_STEAL_RUNTIME: RefCell<Option<NoStealRuntimeThreadData>> = const { RefCell::new(None) };
}

struct Guard(Option<NoStealRuntimeThreadData>);

impl Guard {
    fn new(data: Option<NoStealRuntimeThreadData>) -> Self {
        Self(NO_STEAL_RUNTIME.replace(data))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        NO_STEAL_RUNTIME.with(|data| {
            data.replace(self.0.take());
        });
    }
}

impl NoStealRuntime {
    pub fn new(mut thread_count: usize, name: &str) -> Result<Arc<Self>, std::io::Error> {
        if thread_count == 0 {
            thread_count = num_cpus::get();
        }

        let this = Arc::new(Self {
            runtimes: Vec::new(),
        });

        let runtimes = (0..thread_count)
            .map(|i| {
                let pool = this.clone();

                let init_fn = move || {
                    let pool = pool.clone();
                    NO_STEAL_RUNTIME.with(move |data| {
                        data.replace(Some(NoStealRuntimeThreadData {
                            runtime: pool,
                            idx: i,
                        }))
                    });
                };

                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .thread_name(format!("{name}-{i}"))
                    .on_thread_start(init_fn)
                    .enable_all()
                    .build()
            })
            .collect::<Result<Vec<_>, _>>()?;

        // This is safe because no one is using the runtimes yet
        unsafe {
            let ptr = Arc::as_ptr(&this) as *mut NoStealRuntime;
            let this = &mut *ptr;
            this.runtimes = runtimes;
        }

        Ok(this)
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let idx = rand::thread_rng().gen_range(0..self.runtimes.len());
        self.runtimes[idx].spawn(future)
    }

    pub fn block_on<F>(self: &Arc<Self>, future: F) -> F::Output
    where
        F: Future,
    {
        let _guard = Guard::new(Some(NoStealRuntimeThreadData {
            runtime: self.clone(),
            idx: 0,
        }));

        self.runtimes[0].block_on(future)
    }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    NO_STEAL_RUNTIME.with_borrow(|data| match data {
        Some(data) => data.runtime.spawn(future),
        None => tokio::spawn(future),
    })
}

pub fn current_handle() -> tokio::runtime::Handle {
    NO_STEAL_RUNTIME.with_borrow(|data| match data.as_ref() {
        Some(data) => data.runtime.runtimes[data.idx].handle().clone(),
        None => tokio::runtime::Handle::current(),
    })
}
