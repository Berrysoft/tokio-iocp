use crate::{
    buf::*,
    net::{Socket, *},
    *,
};
use std::net::{Shutdown, SocketAddr, ToSocketAddrs};
use windows_sys::Win32::Networking::WinSock::{IPPROTO_TCP, SOCK_STREAM, SOMAXCONN};

/// A TCP socket server, listening for connections.
///
/// You can accept a new connection by using the [`accept`](`TcpListener::accept`)
/// method.
///
/// # Examples
///
/// ```
/// use tokio_iocp::net::{TcpListener, TcpStream};
/// use std::net::SocketAddr;
///
/// let addr: SocketAddr = "127.0.0.1:2345".parse().unwrap();
///
/// let listener = TcpListener::bind(addr).unwrap();
///
/// tokio_iocp::start(async move {
///     let tx_fut = TcpStream::connect(addr);
///
///     let rx_fut = listener.accept();
///
///     let (tx, (rx, _)) = tokio::try_join!(tx_fut, rx_fut).unwrap();
///
///     tx.send("test").await.0.unwrap();
///
///     let (_, buf) = rx.recv(vec![0; 4]).await;
///
///     assert_eq!(buf, b"test");
/// });
/// ```
pub struct TcpListener {
    inner: Socket,
}

impl TcpListener {
    /// Creates a new `TcpListener`, which will be bound to the specified address.
    ///
    /// The returned listener is ready for accepting connections.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port
    /// to this listener.
    pub fn bind(addr: impl ToSocketAddrs) -> IoResult<Self> {
        super::each_addr(addr, |addr| {
            let socket = Socket::bind(addr, SOCK_STREAM, IPPROTO_TCP)?;
            socket.listen(SOMAXCONN as _)?;
            Ok(Self { inner: socket })
        })
    }

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding [`TcpStream`] and the remote peer's
    /// address will be returned.
    pub async fn accept(&self) -> IoResult<(TcpStream, SocketAddr)> {
        let (socket, addr) = self.inner.accept(SOCK_STREAM, IPPROTO_TCP).await?;
        let stream = TcpStream { inner: socket };
        Ok((stream, addr))
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to
    /// figure out which port was actually bound.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
    /// use tokio_iocp::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    ///
    /// let addr = listener.local_addr().expect("Couldn't get local address");
    /// assert_eq!(addr, SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080)));
    /// ```
    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr()
    }
}

impl_socket!(TcpListener, inner);

/// A TCP stream between a local and a remote socket.
///
/// A TCP stream can either be created by connecting to an endpoint, via the
/// `connect` method, or by accepting a connection from a listener.
///
/// # Examples
///
/// ```no_run
/// use tokio_iocp::net::TcpStream;
/// use std::net::SocketAddr;
///
/// tokio_iocp::start(async {
///     // Connect to a peer
///     let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
///
///     // Write some data.
///     let (result, _) = stream.send("hello world!").await;
///     result.unwrap();
/// })
/// ```
pub struct TcpStream {
    inner: Socket,
}

impl TcpStream {
    /// Opens a TCP connection to a remote host.
    pub async fn connect(addr: impl ToSocketAddrs) -> IoResult<Self> {
        super::each_addr_async(addr, |addr| async move {
            let socket = Socket::bind_any_like(addr, SOCK_STREAM, IPPROTO_TCP)?;
            socket.connect_ex(addr).await?;
            Ok(Self { inner: socket })
        })
        .await
    }

    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> IoResult<SocketAddr> {
        self.inner.peer_addr()
    }

    /// Returns the socket address of the local half of this TCP connection.
    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr()
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O on the specified
    /// portions to return immediately with an appropriate value (see the
    /// documentation of [`Shutdown`]).
    pub fn shutdown(&self, how: Shutdown) -> IoResult<()> {
        self.inner.shutdown(how)
    }

    /// Receives a packet of data from the socket into the buffer, returning the original buffer and
    /// quantity of data received.
    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.recv(buffer).await
    }

    /// Receives a packet of data from the socket into the buffer, returning the original buffer and
    /// quantity of data received.
    pub async fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.recv_vectored(buffer).await
    }

    /// Sends some data to the socket from the buffer, returning the original buffer and
    /// quantity of data sent.
    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        self.inner.send(buffer).await
    }

    /// Sends some data to the socket from the buffer, returning the original buffer and
    /// quantity of data sent.
    pub async fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        self.inner.send_vectored(buffer).await
    }
}

impl_socket!(TcpStream, inner);
