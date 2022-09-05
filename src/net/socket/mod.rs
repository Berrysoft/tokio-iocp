mod io;

use crate::{
    buf::*,
    io_port::IO_PORT,
    op::{recv::*, send::*},
    *,
};
use std::{
    net::SocketAddr,
    ops::Deref,
    os::windows::{
        io::{AsSocket, FromRawSocket, OwnedSocket},
        prelude::AsRawSocket,
    },
    sync::OnceLock,
};
use windows_sys::Win32::Networking::WinSock::{
    bind, connect, socket, WSACleanup, WSAData, WSAGetLastError, WSAStartup, ADDRESS_FAMILY,
    AF_INET, AF_INET6, INVALID_SOCKET, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
};

use self::io::SocketAsyncIo;

struct WSAInit;

impl WSAInit {
    pub fn init() -> Self {
        let mut data: WSAData = unsafe { std::mem::zeroed() };
        let ret = unsafe {
            WSAStartup(
                0x202, // version 2.2
                &mut data,
            )
        };
        assert_eq!(ret, 0);
        Self
    }
}

impl Drop for WSAInit {
    fn drop(&mut self) {
        unsafe { WSACleanup() };
    }
}

static WSA_INIT: OnceLock<WSAInit> = OnceLock::new();

pub struct Socket {
    handle: OwnedSocket,
}

const fn get_domain(addr: SocketAddr) -> ADDRESS_FAMILY {
    match addr {
        SocketAddr::V4(_) => AF_INET,
        SocketAddr::V6(_) => AF_INET6,
    }
}

impl Socket {
    fn new(family: ADDRESS_FAMILY, ty: i32) -> IoResult<Self> {
        WSA_INIT.get_or_init(WSAInit::init);

        let handle = unsafe { socket(family as _, ty, 0) };
        if handle != INVALID_SOCKET {
            let socket = Self {
                handle: unsafe { OwnedSocket::from_raw_socket(handle as _) },
            };
            socket.attach()?;
            Ok(socket)
        } else {
            Err(IoError::from_raw_os_error(unsafe { WSAGetLastError() }))
        }
    }

    fn attach(&self) -> IoResult<()> {
        IO_PORT.attach(self.handle.as_raw_socket() as _)
    }

    fn exact_addr<T>(addr: SocketAddr, f: impl FnOnce(*const SOCKADDR, i32) -> T) -> T {
        match addr {
            SocketAddr::V4(addr) => {
                let native_addr = unsafe {
                    SOCKADDR_IN {
                        sin_family: AF_INET as _,
                        sin_port: addr.port(),
                        sin_addr: std::mem::transmute(addr.ip().octets()),
                        sin_zero: std::mem::zeroed(),
                    }
                };
                f(
                    std::ptr::addr_of!(native_addr) as _,
                    std::mem::size_of_val(&native_addr) as _,
                )
            }
            SocketAddr::V6(addr) => {
                let native_addr = unsafe {
                    SOCKADDR_IN6 {
                        sin6_family: AF_INET6 as _,
                        sin6_port: addr.port(),
                        sin6_flowinfo: 0,
                        sin6_addr: std::mem::transmute(addr.ip().octets()),
                        Anonymous: std::mem::zeroed(),
                    }
                };
                f(
                    std::ptr::addr_of!(native_addr) as _,
                    std::mem::size_of_val(&native_addr) as _,
                )
            }
        }
    }

    pub fn bind(addr: SocketAddr, ty: u16) -> IoResult<Self> {
        let socket = Self::new(get_domain(addr), ty as _)?;
        let res = Self::exact_addr(addr, |addr, len| unsafe {
            bind(socket.as_raw_socket() as _, addr, len)
        });
        if res == 0 {
            Ok(socket)
        } else {
            Err(IoError::from_raw_os_error(unsafe { WSAGetLastError() }))
        }
    }

    pub fn connect(&self, addr: SocketAddr) -> IoResult<()> {
        let res = Self::exact_addr(addr, |addr, len| unsafe {
            connect(self.handle.as_raw_socket() as _, addr, len)
        });
        if res == 0 {
            Ok(())
        } else {
            Err(IoError::from_raw_os_error(unsafe { WSAGetLastError() }))
        }
    }

    pub fn recv<T: IoBufMut>(&self, buffer: T) -> SocketAsyncIo<Recv<T>> {
        SocketAsyncIo::new(self.as_socket(), Recv::new(buffer))
    }

    pub fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> SocketAsyncIo<RecvVectored<T>> {
        SocketAsyncIo::new(self.as_socket(), RecvVectored::new(buffer))
    }

    pub fn send<T: IoBuf>(&self, buffer: T) -> SocketAsyncIo<Send<T>> {
        SocketAsyncIo::new(self.as_socket(), Send::new(buffer))
    }

    pub fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> SocketAsyncIo<SendVectored<T>> {
        SocketAsyncIo::new(self.as_socket(), SendVectored::new(buffer))
    }
}

impl Deref for Socket {
    type Target = OwnedSocket;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
