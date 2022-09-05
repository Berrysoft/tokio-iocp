mod io;

use crate::{buf::*, op::net::*, *};
use std::{
    net::SocketAddr,
    ops::Deref,
    os::windows::io::{AsSocket, FromRawSocket, OwnedSocket},
    sync::OnceLock,
};
use windows_sys::Win32::Networking::WinSock::{
    socket, WSACleanup, WSAData, WSAGetLastError, WSAStartup, ADDRESS_FAMILY, AF_INET, AF_INET6,
    INVALID_SOCKET,
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
    pub fn new(addr: SocketAddr, ty: i32) -> IoResult<Self> {
        WSA_INIT.get_or_init(WSAInit::init);

        let handle = unsafe { socket(get_domain(addr) as i32, ty, 0) };
        if handle != INVALID_SOCKET {
            Ok(Self {
                handle: unsafe { OwnedSocket::from_raw_socket(handle as _) },
            })
        } else {
            Err(IoError::from_raw_os_error(unsafe { WSAGetLastError() }))
        }
    }

    pub fn recv<T: IoBufMut>(&self, buffer: T) -> SocketAsyncIo<Recv<T>> {
        SocketAsyncIo::new(self.as_socket(), buffer, Recv::default())
    }

    pub fn send<T: IoBuf>(&self, buffer: T) -> SocketAsyncIo<Send<T>> {
        SocketAsyncIo::new(self.as_socket(), buffer, Send::default())
    }
}

impl Deref for Socket {
    type Target = OwnedSocket;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
