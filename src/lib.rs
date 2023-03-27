//! Tokio-iocp provides a safe [IOCP] interface for the Tokio runtime.
//!
//! [IOCP]: https://docs.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports
//!
//! # Getting started
//!
//! Using `tokio-iocp` requires starting a `tokio-iocp` runtime. This
//! runtime internally manages the main Tokio runtime and a IOCP handle.
//!
//! ```
//! use tokio_iocp::{fs::File, IoResult};
//!
//! fn main() -> IoResult<()> {
//!     tokio_iocp::start(async {
//!         // Open a file
//!         let file = File::open("Cargo.toml")?;
//!
//!         let buf = Vec::with_capacity(4096);
//!         // Read some data, the buffer is passed by ownership and
//!         // submitted to the kernel. When the operation completes,
//!         // we get the buffer back.
//!         let (res, buf) = file.read_at(buf, 0).await;
//!         let n = res?;
//!
//!         // Display the contents
//!         println!("{}", String::from_utf8_lossy(&buf));
//!
//!         Ok(())
//!     })
//! }
//! ```
//! Under the hood, `tokio_iocp::start` starts a current-thread Runtime.
//! For concurrency, spawn multiple threads, each with a `tokio-iocp` runtime.
//!
//!
//! # Submit-based operations
//!
//! Unlike Tokio proper, IOCP needs the ownership of resources.
//! Ownership of resources are passed to the kernel, which then performs the
//! operation. When the operation completes, ownership is passed back to the
//! caller. Because of this difference, the `tokio-iocp` APIs diverge.
//!
//! For example, in the above example, reading from a `File` requires passing
//! ownership of the buffer.

#![cfg_attr(feature = "read_buf", feature(read_buf))]
#![warn(missing_docs)]

pub mod buf;
pub mod fs;
mod io_port;
pub mod net;
mod op;
pub mod runtime;

#[doc(no_inline)]
pub use runtime::spawn;
#[doc(no_inline)]
pub use std::io::{Error as IoError, Result as IoResult};

/// Start an IOCP enabled Tokio runtime.
///
/// All `tokio-iocp` resource types must be used from within the context of a
/// runtime. The `start` method initializes the runtime and runs it for the
/// duration of `future`.
///
/// The `tokio-iocp` runtime is compatible with all Tokio, so it is possible to
/// run Tokio based libraries (e.g. hyper) from within the tokio-iocp runtime.
/// A `tokio-iocp` runtime consists of a Tokio `current_thread` runtime.
/// All tasks spawned on the `tokio-iocp` runtime are executed on the current thread.
/// To add concurrency, spawn multiple threads, each with a `tokio-iocp` runtime.
pub fn start<F: std::future::Future>(future: F) -> F::Output {
    runtime::Runtime::new().unwrap().block_on(future)
}

/// A specialized `Result` type for IOCP operations with buffers.
///
/// This type is used as a return value for asynchronous IOCP methods that
/// require passing ownership of a buffer to the runtime. When the operation
/// completes, the buffer is returned whether or not the operation completed
/// successfully.
pub type BufResult<T, B> = (IoResult<T>, B);
