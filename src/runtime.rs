//! The runtime of Tokio with IOCP.

use crate::{io_port::IO_PORT, *};
use std::future::Future;
use tokio::task::{JoinHandle, LocalSet};

/// The `tokio-iocp` runtime.
#[derive(Debug)]
pub struct Runtime {
    rt: tokio::runtime::Runtime,
    local: LocalSet,
}

impl Runtime {
    /// Creates a new Tokio runtime, with all features enabled.
    pub fn new() -> IoResult<Self> {
        Ok(Self {
            rt: tokio::runtime::Builder::new_current_thread()
                .on_thread_park(|| IO_PORT.with(|port| port.poll()))
                .enable_all()
                .build()?,
            local: LocalSet::new(),
        })
    }

    /// Runs a future to completion on the runtime.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.local.block_on(&self.rt, future)
    }
}

/// Spawns a new asynchronous task, returning a [`JoinHandle`] for it.
///
/// Spawning a task enables the task to execute concurrently to other tasks.
/// There is no guarantee that a spawned task will execute to completion. When a
/// runtime is shutdown, all outstanding tasks are dropped, regardless of the
/// lifecycle of that task.
///
/// This function must be called from the context of a `tokio-uring` runtime.
///
/// [`JoinHandle`]: tokio::task::JoinHandle
///
/// # Examples
///
/// In this example, a server is started and `spawn` is used to start a new task
/// that processes each received connection.
///
/// ```
/// tokio_iocp::start(async {
///     let handle = tokio_iocp::spawn(async {
///         println!("hello from a background task");
///     });
///
///     // Let the task complete
///     handle.await.unwrap();
/// });
/// ```
pub fn spawn<F: Future + 'static>(future: F) -> JoinHandle<F::Output> {
    tokio::task::spawn_local(future)
}

#[cfg(feature = "criterion")]
impl criterion::async_executor::AsyncExecutor for Runtime {
    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        self.block_on(future)
    }
}

#[cfg(feature = "criterion")]
impl criterion::async_executor::AsyncExecutor for &Runtime {
    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        (*self).block_on(future)
    }
}
