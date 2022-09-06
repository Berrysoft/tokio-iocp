use crate::{buf::*, net::socket::Socket, *};
use std::net::SocketAddr;
use windows_sys::Win32::Networking::WinSock::{IPPROTO_TCP, SOCK_STREAM, SOMAXCONN};

pub struct TcpListener {
    inner: Socket,
}

impl TcpListener {
    pub fn bind(addr: impl Into<SocketAddr>) -> IoResult<Self> {
        let socket = Socket::bind(addr.into(), SOCK_STREAM, IPPROTO_TCP)?;
        socket.listen(SOMAXCONN as _)?;
        Ok(Self { inner: socket })
    }

    pub async fn accept(&self) -> IoResult<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.inner.accept(SOCK_STREAM, IPPROTO_TCP).await?;
        let stream = TcpStream { inner: socket };
        Ok((stream, addr))
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr()
    }
}

pub struct TcpStream {
    inner: Socket,
}

impl TcpStream {
    pub fn connect(addr: impl Into<SocketAddr>) -> IoResult<Self> {
        let addr = addr.into();
        let socket = Socket::new(addr, SOCK_STREAM, IPPROTO_TCP)?;
        socket.connect(addr)?;
        Ok(Self { inner: socket })
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr()
    }

    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.recv(buffer).await
    }

    pub async fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.recv_vectored(buffer).await
    }

    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.send(buffer).await
    }

    pub async fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.send_vectored(buffer).await
    }
}
