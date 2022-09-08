use crate::{
    buf::*,
    io_port::{IocpFuture, IO_PORT},
    net::{UnixSocketAddr, *},
    op::{accept::*, connect::*, recv::*, recv_from::*, send::*, send_to::*},
    *,
};
use once_cell::sync::OnceCell as OnceLock;
use std::{
    net::{Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, SocketAddrV4, SocketAddrV6},
    os::windows::prelude::{AsRawSocket, AsSocket, FromRawSocket, OwnedSocket},
};
use windows_sys::Win32::Networking::WinSock::{
    bind, connect, getpeername, getsockname, listen, shutdown, sockaddr_un, socket, WSACleanup,
    WSAData, WSAStartup, ADDRESS_FAMILY, AF_INET, AF_INET6, AF_UNIX, INVALID_SOCKET, IPPROTO,
    SD_BOTH, SD_RECEIVE, SD_SEND, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_STORAGE, SOCKET,
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

impl Socket {
    pub fn new(addr: ADDRESS_FAMILY, ty: u16, protocol: IPPROTO) -> IoResult<Self> {
        WSA_INIT.get_or_init(WSAInit::init);

        let handle = unsafe { socket(addr as _, ty as _, protocol) };
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

    pub fn bind(addr: impl SockAddr, ty: u16, protocol: IPPROTO) -> IoResult<Self> {
        let socket = Self::new(addr.domain() as _, ty as _, protocol)?;
        let res =
            unsafe { addr.with_native(|addr, len| bind(socket.as_raw_socket() as _, addr, len)) };
        if res == 0 {
            Ok(socket)
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub fn bind_any_like(addr: SocketAddr, ty: u16, protocol: IPPROTO) -> IoResult<Self> {
        let new_addr: SocketAddr = match addr {
            SocketAddr::V4(addr) => SocketAddrV4::new(*addr.ip(), 0).into(),
            SocketAddr::V6(addr) => SocketAddrV6::new(*addr.ip(), 0, 0, 0).into(),
        };
        Self::bind(new_addr, ty, protocol)
    }

    pub fn connect(&self, addr: impl SockAddr) -> IoResult<()> {
        let res =
            unsafe { addr.with_native(|addr, len| connect(self.as_raw_socket() as _, addr, len)) };
        if res == 0 {
            Ok(())
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub async fn connect_ex(&self, addr: impl SockAddr) -> IoResult<()> {
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

    fn get_addr<A: SockAddr>(
        &self,
        f: unsafe extern "system" fn(SOCKET, *mut SOCKADDR, *mut i32) -> i32,
    ) -> IoResult<A> {
        let mut name = [0u8; MAX_ADDR_SIZE];
        let mut namelen: i32 = MAX_ADDR_SIZE as _;
        let res = unsafe {
            f(
                self.as_raw_socket() as _,
                name.as_mut_ptr() as _,
                &mut namelen,
            )
        };
        if res == 0 {
            Ok(unsafe { A::try_from_native(name.as_ptr() as _, namelen as _).unwrap() })
        } else {
            Err(IoError::last_os_error())
        }
    }

    pub fn peer_addr<A: SockAddr>(&self) -> IoResult<A> {
        self.get_addr(getpeername)
    }

    pub fn local_addr<A: SockAddr>(&self) -> IoResult<A> {
        self.get_addr(getsockname)
    }

    pub async fn accept<A: SockAddr>(&self, ty: u16, protocol: IPPROTO) -> IoResult<(Socket, A)> {
        let local_addr: A = self.local_addr()?;
        let accept_socket = Socket::new(local_addr.domain() as _, ty, protocol)?;
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
        IocpFuture::new(self.as_socket(), RecvFromOne::<_, SocketAddr>::new(buffer)).await
    }

    pub async fn recv_from_vectored<T: IoBufMut>(
        &self,
        buffer: Vec<T>,
    ) -> BufResult<(usize, SocketAddr), Vec<T>> {
        IocpFuture::new(
            self.as_socket(),
            RecvFromVectored::<_, SocketAddr>::new(buffer),
        )
        .await
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

impl_socket!(Socket, handle);

pub const MAX_ADDR_SIZE: usize = std::mem::size_of::<SOCKADDR_STORAGE>();

pub trait SockAddr: Sized + Unpin {
    fn domain(&self) -> u16;

    unsafe fn try_from_native(addr: *const SOCKADDR, len: i32) -> Option<Self>;

    unsafe fn with_native<T>(&self, f: impl FnOnce(*const SOCKADDR, i32) -> T) -> T;
}

impl SockAddr for SocketAddr {
    fn domain(&self) -> u16 {
        match self {
            Self::V4(_) => AF_INET as _,
            Self::V6(_) => AF_INET6 as _,
        }
    }

    unsafe fn try_from_native(addr: *const SOCKADDR, _len: i32) -> Option<Self> {
        let addr_ref = addr.as_ref().unwrap();
        match addr_ref.sa_family as u32 {
            AF_INET => {
                let addr = (addr as *const SOCKADDR_IN).as_ref().unwrap();
                Some(SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::from(addr.sin_addr.S_un.S_addr.to_ne_bytes()),
                    addr.sin_port,
                )))
            }
            AF_INET6 => {
                let addr = (addr as *const SOCKADDR_IN6).as_ref().unwrap();
                Some(SocketAddr::V6(SocketAddrV6::new(
                    Ipv6Addr::from(addr.sin6_addr.u.Byte),
                    addr.sin6_port,
                    addr.sin6_flowinfo,
                    addr.Anonymous.sin6_scope_id,
                )))
            }
            _ => None,
        }
    }

    unsafe fn with_native<T>(&self, f: impl FnOnce(*const SOCKADDR, i32) -> T) -> T {
        match self {
            SocketAddr::V4(addr) => {
                let native_addr = SOCKADDR_IN {
                    sin_family: AF_INET as _,
                    sin_port: addr.port(),
                    sin_addr: std::mem::transmute(u32::from_ne_bytes(addr.ip().octets())),
                    sin_zero: std::mem::zeroed(),
                };
                f(
                    std::ptr::addr_of!(native_addr) as _,
                    std::mem::size_of_val(&native_addr) as _,
                )
            }
            SocketAddr::V6(addr) => {
                let native_addr = SOCKADDR_IN6 {
                    sin6_family: AF_INET6 as _,
                    sin6_port: addr.port(),
                    sin6_flowinfo: 0,
                    sin6_addr: std::mem::transmute(addr.ip().octets()),
                    Anonymous: std::mem::zeroed(),
                };
                f(
                    std::ptr::addr_of!(native_addr) as _,
                    std::mem::size_of_val(&native_addr) as _,
                )
            }
        }
    }
}

impl SockAddr for UnixSocketAddr {
    fn domain(&self) -> u16 {
        AF_UNIX
    }

    unsafe fn try_from_native(addr: *const SOCKADDR, len: i32) -> Option<Self> {
        let addr_ref = addr.as_ref().unwrap();
        if addr_ref.sa_family == AF_UNIX {
            let addr = (addr as *const sockaddr_un).as_ref().unwrap();
            let len = (len - 2) as usize;
            Some(UnixSocketAddr {
                path: addr.sun_path,
                len,
            })
        } else {
            None
        }
    }

    unsafe fn with_native<T>(&self, f: impl FnOnce(*const SOCKADDR, i32) -> T) -> T {
        let addr = sockaddr_un {
            sun_family: AF_UNIX,
            sun_path: self.path,
        };
        f(&addr as *const _ as _, (self.len + 2) as _)
    }
}
