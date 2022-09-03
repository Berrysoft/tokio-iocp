use crate::{io_port::IO_PORT, *};
use std::future::Future;

#[derive(Debug)]
pub struct Runtime {
    rt: tokio::runtime::Runtime,
}

impl Runtime {
    pub fn new() -> IoResult<Self> {
        Ok(Self {
            rt: tokio::runtime::Builder::new_current_thread()
                .on_thread_park(|| IO_PORT.poll().unwrap())
                .enable_all()
                .build()?,
        })
    }

    pub fn block_on<F: Future>(&mut self, future: F) -> F::Output {
        self.rt.block_on(future)
    }
}

pub fn spawn<F: Future + 'static>(future: F) -> tokio::task::JoinHandle<F::Output> {
    tokio::task::spawn_local(future)
}
