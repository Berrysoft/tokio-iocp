use crate::{
    buf::*,
    io_port::{socket::*, IO_PORT},
    op::{recv::*, send::*, send_to::*, wsa_exact_addr},
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
    AF_INET, AF_INET6, INVALID_SOCKET,
};

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

    pub fn bind(addr: SocketAddr, ty: u16) -> IoResult<Self> {
        let socket = Self::new(get_domain(addr), ty as _)?;
        let res = unsafe {
            wsa_exact_addr(addr, |addr, len| {
                bind(socket.as_raw_socket() as _, addr, len)
            })
        };
        if res == 0 {
            Ok(socket)
        } else {
            Err(IoError::from_raw_os_error(unsafe { WSAGetLastError() }))
        }
    }

    pub fn connect(&self, addr: SocketAddr) -> IoResult<()> {
        let res = unsafe {
            wsa_exact_addr(addr, |addr, len| {
                connect(self.handle.as_raw_socket() as _, addr, len)
            })
        };
        if res == 0 {
            Ok(())
        } else {
            Err(IoError::from_raw_os_error(unsafe { WSAGetLastError() }))
        }
    }

    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        SocketFuture::new(self.as_socket(), Recv::new(buffer)).await
    }

    pub async fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        SocketFuture::new(self.as_socket(), RecvVectored::new(buffer)).await
    }

    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        SocketFuture::new(self.as_socket(), Send::new(buffer)).await
    }

    pub async fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        SocketFuture::new(self.as_socket(), SendVectored::new(buffer)).await
    }

    pub async fn send_to<T: IoBuf>(&self, buffer: T, addr: SocketAddr) -> BufResult<usize, T> {
        SocketFuture::new(self.as_socket(), SendTo::new(buffer, addr)).await
    }

    pub async fn send_to_vectored<T: IoBuf>(
        &self,
        buffer: Vec<T>,
        addr: SocketAddr,
    ) -> BufResult<usize, Vec<T>> {
        SocketFuture::new(self.as_socket(), SendToVectored::new(buffer, addr)).await
    }
}

impl Deref for Socket {
    type Target = OwnedSocket;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
