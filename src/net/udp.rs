use crate::{buf::*, net::socket::Socket, *};
use std::net::SocketAddr;
use windows_sys::Win32::Networking::WinSock::{IPPROTO_UDP, SOCK_DGRAM};

/// A UDP socket.
///
/// UDP is "connectionless", unlike TCP. Meaning, regardless of what address you've bound to, a `UdpSocket`
/// is free to communicate with many different remotes. In tokio there are basically two main ways to use `UdpSocket`:
///
/// * one to many: [`bind`](`UdpSocket::bind`) and use [`send_to`](`UdpSocket::send_to`)
///   and [`recv_from`](`UdpSocket::recv_from`) to communicate with many different addresses
/// * one to one: [`connect`](`UdpSocket::connect`) and associate with a single address, using [`send`](`UdpSocket::send`)
///   and [`recv`](`UdpSocket::recv`) to communicate only with that remote address
///
/// # Examples
/// Bind and connect a pair of sockets and send a packet:
///
/// ```
/// use tokio_iocp::{net::UdpSocket, IoResult};
/// use std::net::SocketAddr;
/// fn main() -> IoResult<()> {
///     tokio_iocp::start(async {
///         let first_addr: SocketAddr = "127.0.0.1:2401".parse().unwrap();
///         let second_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
///
///         // bind sockets
///         let socket = UdpSocket::bind(first_addr)?;
///         let other_socket = UdpSocket::bind(second_addr)?;
///
///         // connect sockets
///         socket.connect(second_addr)?;
///         other_socket.connect(first_addr)?;
///
///         let buf = Vec::with_capacity(32);
///
///         // write data
///         let (result, _) = socket.send("hello world").await;
///         result?;
///
///         // read data
///         let (result, buf) = other_socket.recv(buf).await;
///         let n_bytes = result?;
///
///         assert_eq!(n_bytes, buf.len());
///         assert_eq!(b"hello world", &buf[..]);
///
///         Ok(())
///     })
/// }
/// ```
/// Send and receive packets without connecting:
///
/// ```
/// use tokio_iocp::{net::UdpSocket, IoResult};
/// use std::net::SocketAddr;
/// fn main() -> IoResult<()> {
///     tokio_iocp::start(async {
///         let first_addr: SocketAddr = "127.0.0.1:2401".parse().unwrap();
///         let second_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
///
///         // bind sockets
///         let socket = UdpSocket::bind(first_addr)?;
///         let other_socket = UdpSocket::bind(second_addr)?;
///
///         let buf = Vec::with_capacity(32);
///
///         // write data
///         let (result, _) = socket.send_to("hello world", second_addr).await;
///         result?;
///
///         // read data
///         let (result, buf) = other_socket.recv_from(buf).await;
///         let (n_bytes, addr) = result?;
///
///         assert_eq!(addr, first_addr);
///         assert_eq!(n_bytes, buf.len());
///         assert_eq!(b"hello world", &buf[..]);
///
///         Ok(())
///     })
/// }
/// ```
pub struct UdpSocket {
    inner: Socket,
}

impl UdpSocket {
    /// Creates a new UDP socket and attempt to bind it to the addr provided.
    pub fn bind(addr: impl Into<SocketAddr>) -> IoResult<Self> {
        Ok(Self {
            inner: Socket::bind(addr.into(), SOCK_DGRAM, IPPROTO_UDP)?,
        })
    }

    /// Connects this UDP socket to a remote address, allowing the `send` and
    /// `recv` to be used to send data and also applies filters to only
    /// receive data from the specified address.
    ///
    /// Note that usually, a successful `connect` call does not specify
    /// that there is a remote server listening on the port, rather, such an
    /// error would only be detected after the first send.
    pub fn connect(&self, addr: impl Into<SocketAddr>) -> IoResult<()> {
        self.inner.connect(addr.into())
    }

    /// Returns the local address that this socket is bound to.
    ///
    /// # Example
    ///
    /// ```
    /// use tokio_iocp::{net::UdpSocket, IoResult};
    /// use std::net::SocketAddr;
    ///
    /// # fn main() -> IoResult<()> {
    /// # tokio_iocp::start(async {
    /// let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    /// let sock = UdpSocket::bind(addr)?;
    /// // the address the socket is bound to
    /// let local_addr = sock.local_addr()?;
    /// assert_eq!(local_addr, addr);
    /// # Ok(())
    /// # })
    /// # }
    /// ```
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

    pub async fn recv_from<T: IoBufMut>(&self, buffer: T) -> BufResult<(usize, SocketAddr), T> {
        self.inner.recv_from(buffer).await
    }

    pub async fn recv_from_vectored<T: IoBufMut>(
        &self,
        buffer: Vec<T>,
    ) -> BufResult<(usize, SocketAddr), Vec<T>> {
        self.inner.recv_from_vectored(buffer).await
    }

    pub async fn send_to<T: IoBuf>(&self, buffer: T, addr: SocketAddr) -> BufResult<usize, T> {
        self.inner.send_to(buffer, addr).await
    }

    pub async fn send_to_vectored<T: IoBuf>(
        &self,
        buffer: Vec<T>,
        addr: SocketAddr,
    ) -> BufResult<usize, Vec<T>> {
        self.inner.send_to_vectored(buffer, addr).await
    }
}
