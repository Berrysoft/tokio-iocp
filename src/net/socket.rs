use crate::{
    buf::*,
    io_port::{IocpFuture, IO_PORT},
    op::{
        accept::*, connect::*, recv::*, recv_from::*, send::*, send_to::*, wsa_exact_addr,
        wsa_get_addr, MAX_ADDR_SIZE,
    },
    *,
};
use once_cell::sync::OnceCell as OnceLock;
use std::{
    net::{Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6},
    ops::Deref,
    os::windows::{
        io::{AsSocket, FromRawSocket, OwnedSocket},
        prelude::AsRawSocket,
    },
    ptr::null,
};
use windows_sys::Win32::Networking::WinSock::{
    bind, connect, getsockname, listen, shutdown, WSACleanup, WSAData, WSASocketW, WSAStartup,
    ADDRESS_FAMILY, AF_INET, AF_INET6, INVALID_SOCKET, IPPROTO, SD_BOTH, SD_RECEIVE, SD_SEND,
    WSA_FLAG_OVERLAPPED,
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
    pub fn new(addr: SocketAddr, ty: u16, protocol: IPPROTO) -> IoResult<Self> {
        WSA_INIT.get_or_init(WSAInit::init);

        let handle = unsafe {
            WSASocketW(
                get_domain(addr) as _,
                ty as _,
                protocol,
                null(),
                0,
                WSA_FLAG_OVERLAPPED,
            )
        };
        if handle != INVALID_SOCKET {
            let socket = Self {
                handle: unsafe { OwnedSocket::from_raw_socket(handle as _) },
            };
            socket.attach()?;
            Ok(socket)
        } else {
            Err(IoError::last_os_error())
        }
    }

    fn attach(&self) -> IoResult<()> {
        IO_PORT.with(|port| port.attach(self.as_raw_socket() as _))
    }

    pub fn bind(addr: SocketAddr, ty: u16, protocol: IPPROTO) -> IoResult<Self> {
        let socket = Self::new(addr, ty as _, protocol)?;
        let res = unsafe {
            wsa_exact_addr(addr, |addr, len| {
                bind(socket.as_raw_socket() as _, addr, len)
            })
        };
        if res == 0 {
            Ok(socket)
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub fn bind_any_like(addr: SocketAddr, ty: u16, protocol: IPPROTO) -> IoResult<Self> {
        let new_addr = match addr {
            SocketAddr::V4(addr) => SocketAddrV4::new(*addr.ip(), 0).into(),
            SocketAddr::V6(addr) => SocketAddrV6::new(*addr.ip(), 0, 0, 0).into(),
        };
        Self::bind(new_addr, ty, protocol)
    }

    pub fn connect(&self, addr: SocketAddr) -> IoResult<()> {
        let res = unsafe {
            wsa_exact_addr(addr, |addr, len| {
                connect(self.as_raw_socket() as _, addr, len)
            })
        };
        if res == 0 {
            Ok(())
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub async fn connect_ex(&self, addr: SocketAddr) -> IoResult<()> {
        println!("Connect {}", addr);
        IocpFuture::new(self.as_socket(), Connect::new(addr))
            .await
            .0
    }

    pub fn listen(&self, backlog: i32) -> IoResult<()> {
        let res = unsafe { listen(self.as_raw_socket() as _, backlog) };
        if res == 0 {
            Ok(())
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        let mut name = [0u8; MAX_ADDR_SIZE];
        let mut namelen: i32 = MAX_ADDR_SIZE as _;
        let res = unsafe {
            getsockname(
                self.as_raw_socket() as _,
                name.as_mut_ptr() as _,
                &mut namelen,
            )
        };
        if res == 0 {
            Ok(unsafe { wsa_get_addr(name.as_ptr() as _, namelen as _) })
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub async fn accept(&self, ty: u16, protocol: IPPROTO) -> IoResult<(Socket, SocketAddr)> {
        let local_addr = self.local_addr()?;
        let accept_socket = Socket::new(local_addr, ty, protocol)?;
        let (res, accept_socket) =
            IocpFuture::new(self.as_socket(), Accept::new(accept_socket.handle)).await;
        let addr = res?;
        Ok((
            Socket {
                handle: accept_socket,
            },
            addr,
        ))
    }

    pub fn shutdown(&self, how: Shutdown) -> IoResult<()> {
        let how = match how {
            Shutdown::Write => SD_SEND,
            Shutdown::Read => SD_RECEIVE,
            Shutdown::Both => SD_BOTH,
        };
        let res = unsafe { shutdown(self.handle.as_raw_socket() as _, how as _) };
        if res == 0 {
            Ok(())
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        IocpFuture::new(self.as_socket(), RecvOne::new(buffer)).await
    }

    pub async fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        IocpFuture::new(self.as_socket(), RecvVectored::new(buffer)).await
    }

    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        IocpFuture::new(self.as_socket(), SendOne::new(buffer)).await
    }

    pub async fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        IocpFuture::new(self.as_socket(), SendVectored::new(buffer)).await
    }

    pub async fn recv_from<T: IoBufMut>(&self, buffer: T) -> BufResult<(usize, SocketAddr), T> {
        IocpFuture::new(self.as_socket(), RecvFromOne::new(buffer)).await
    }

    pub async fn recv_from_vectored<T: IoBufMut>(
        &self,
        buffer: Vec<T>,
    ) -> BufResult<(usize, SocketAddr), Vec<T>> {
        IocpFuture::new(self.as_socket(), RecvFromVectored::new(buffer)).await
    }

    pub async fn send_to<T: IoBuf>(&self, buffer: T, addr: SocketAddr) -> BufResult<usize, T> {
        IocpFuture::new(self.as_socket(), SendToOne::new(buffer, addr)).await
    }

    pub async fn send_to_vectored<T: IoBuf>(
        &self,
        buffer: Vec<T>,
        addr: SocketAddr,
    ) -> BufResult<usize, Vec<T>> {
        IocpFuture::new(self.as_socket(), SendToVectored::new(buffer, addr)).await
    }
}

impl Deref for Socket {
    type Target = OwnedSocket;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
