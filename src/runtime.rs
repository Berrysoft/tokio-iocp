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
                .on_thread_park(|| IO_PORT.poll())
                .enable_all()
                .build()?,
            local: LocalSet::new(),
        })
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.local.block_on(&self.rt, future)
    }
}

pub fn spawn<F: Future + 'static>(future: F) -> JoinHandle<F::Output> {
    tokio::task::spawn_local(future)
}
