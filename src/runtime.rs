//! The async runtime the phone's link and the streaming app's wait run on: one
//! multi-threaded tokio runtime for the app's life, started on first use.

use std::{future::Future, sync::OnceLock};

use tokio::{runtime::Runtime, task::JoinHandle};

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("the async runtime starts")
    })
}

/// Runs `task` on the runtime; aborting the handle stops it.
pub fn spawn<F>(task: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(task)
}
