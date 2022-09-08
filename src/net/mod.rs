//! TCP/UDP bindings for IOCP.
//!
//! This module contains the TCP/UDP networking types, similar to the standard
//! library, which can be used to implement networking protocols.
//!
//! # Organization
//!
//! * [`TcpListener`] and [`TcpStream`] provide functionality for communication over TCP
//! * [`UdpSocket`] provides functionality for communication over UDP

mod socket;
mod tcp;
mod udp;

use crate::{IoError, IoResult};
use std::{
    future::Future,
    net::{SocketAddr, ToSocketAddrs},
};

pub use tcp::*;
pub use udp::*;

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
