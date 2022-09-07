use crate::{io_port::IO_PORT, *};
use std::future::Future;
use tokio::task::{JoinHandle, LocalSet};

#[derive(Debug)]
pub struct Runtime {
    rt: tokio::runtime::Runtime,
    local: LocalSet,
}

impl Runtime {
    pub fn new() -> IoResult<Self> {
        Ok(Self {
            rt: tokio::runtime::Builder::new_current_thread()
                .on_thread_park(|| IO_PORT.with(|port| port.poll()))
                .enable_all()
                .build()?,
            local: LocalSet::new(),
        })
    }

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
/// ```no_run
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
