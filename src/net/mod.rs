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

pub use tcp::*;
pub use udp::*;
