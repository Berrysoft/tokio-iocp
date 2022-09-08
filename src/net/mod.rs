//! TCP/UDP bindings for IOCP.
//!
//! This module contains the TCP/UDP networking types, similar to the standard
//! library, which can be used to implement networking protocols.
//!
//! # Organization
//!
//! * [`TcpListener`] and [`TcpStream`] provide functionality for communication over TCP
//! * [`UdpSocket`] provides functionality for communication over UDP
//! * [`UnixListener`] and [`UnixStream`] provide functionality for communication over UNIX sockets.

mod socket;
pub(crate) use socket::*;

mod tcp;
pub use tcp::*;

mod udp;
pub use udp::*;

mod unix;
pub use unix::*;

use crate::{IoError, IoResult};
use std::{
    future::Future,
    net::{SocketAddr, ToSocketAddrs},
};

fn each_addr<T>(
    addr: impl ToSocketAddrs,
    mut f: impl FnMut(SocketAddr) -> IoResult<T>,
) -> IoResult<T> {
    let addrs = addr.to_socket_addrs()?;
    let mut last_err = None;
    for addr in addrs {
        match f(addr) {
            Ok(l) => return Ok(l),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        IoError::new(
            std::io::ErrorKind::InvalidInput,
            "could not resolve to any addresses",
        )
    }))
}

async fn each_addr_async<T, F: Future<Output = IoResult<T>>>(
    addr: impl ToSocketAddrs,
    mut f: impl FnMut(SocketAddr) -> F,
) -> IoResult<T> {
    let addrs = addr.to_socket_addrs()?;
    let mut last_err = None;
    for addr in addrs {
        match f(addr).await {
            Ok(l) => return Ok(l),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        IoError::new(
            std::io::ErrorKind::InvalidInput,
            "could not resolve to any addresses",
        )
    }))
}

macro_rules! impl_socket {
    ($t:ty, $inner:ident) => {
        impl ::std::os::windows::io::AsRawSocket for $t {
            fn as_raw_socket(&self) -> ::std::os::windows::io::RawSocket {
                self.$inner.as_raw_socket()
            }
        }
        impl ::std::os::windows::io::IntoRawSocket for $t {
            fn into_raw_socket(self) -> ::std::os::windows::io::RawSocket {
                self.$inner.into_raw_socket()
            }
        }
        impl ::std::os::windows::io::AsSocket for $t {
            fn as_socket(&self) -> ::std::os::windows::io::BorrowedSocket {
                self.$inner.as_socket()
            }
        }
    };
}

pub(crate) use impl_socket;
