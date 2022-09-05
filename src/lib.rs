#![feature(once_cell)]
#![feature(thread_local)]

pub mod buf;
pub mod fs;
mod io_port;
pub mod net;
mod op;
pub mod runtime;

use std::future::Future;

pub use runtime::spawn;
pub use std::io::{Error as IoError, Result as IoResult};

pub fn start<F: Future>(future: F) -> F::Output {
    runtime::Runtime::new().unwrap().block_on(future)
}

pub type BufResult<T, B> = (IoResult<T>, B);
