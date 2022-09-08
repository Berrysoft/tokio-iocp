use crate::{buf::*, net::Socket, *};
use std::path::{Path, PathBuf};
use windows_sys::Win32::Networking::WinSock::{AF_UNIX, SOCK_STREAM};

/// A Unix socket server, listening for connections.
///
/// You can accept a new connection by using the [`accept`](`UnixListener::accept`)
/// method.
///
/// # Examples
///
/// ```
/// use tokio_iocp::net::{UnixListener, UnixStream};
/// use scopeguard::defer;
///
/// let sock_file = format!("{}/unix-server.sock", std::env::var("TEMP").unwrap());
///
/// tokio_iocp::start(async move {
///     let listener = UnixListener::bind(&sock_file).unwrap();
///     defer! {
///         std::fs::remove_file(&sock_file).unwrap();
///     }
///
///     let tx = UnixStream::connect(&sock_file).unwrap();
///     let rx = listener.accept().await.unwrap();
///
///     tx.send("test").await.0.unwrap();
///
///     let (res, buf) = rx.recv(vec![0; 4]).await;
///     res.unwrap();
///
///     assert_eq!(buf, b"test");
/// });
/// ```
pub struct UnixListener {
    inner: Socket,
}

impl UnixListener {
    /// Creates a new UnixListener, which will be bound to the specified file path.
    /// The file path cannnot yet exist, and will be cleaned up upon dropping `UnixListener`
    pub fn bind(path: impl AsRef<Path>) -> IoResult<UnixListener> {
        let socket = Socket::bind(path.as_ref().to_path_buf(), SOCK_STREAM, 0)?;
        socket.listen(1024)?;
        Ok(UnixListener { inner: socket })
    }

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new Unix domain socket connection
    /// is established. When established, the corresponding [`UnixStream`] and
    /// will be returned.
    pub async fn accept(&self) -> IoResult<UnixStream> {
        let (socket, _) = self.inner.accept::<PathBuf>(SOCK_STREAM, 0).await?;
        let stream = UnixStream { inner: socket };
        Ok(stream)
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_iocp::net::UnixListener;
    /// use std::path::Path;
    /// use scopeguard::defer;
    ///
    /// let sock_file = format!("{}/unix-server.sock", std::env::var("TEMP").unwrap());
    /// let listener = UnixListener::bind(&sock_file).unwrap();
    /// defer! {
    ///     std::fs::remove_file(&sock_file).unwrap();
    /// }
    ///
    /// let addr = listener.local_addr().expect("Couldn't get local address");
    /// assert_eq!(addr, Path::new(&sock_file));
    /// ```
    pub fn local_addr(&self) -> IoResult<PathBuf> {
        self.inner.local_addr()
    }
}

/// A Unix stream between two local sockets on a Unix OS.
///
/// A Unix stream can either be created by connecting to an endpoint, via the
/// `connect` method, or by accepting a connection from a listener.
///
/// # Examples
///
/// ```no_run
/// use tokio_iocp::net::UnixStream;
///
/// tokio_iocp::start(async {
///     // Connect to a peer
///     let mut stream = UnixStream::connect("unix-server.sock").unwrap();
///
///     // Write some data.
///     let (result, _) = stream.send("hello world!").await;
///     result.unwrap();
/// })
/// ```
pub struct UnixStream {
    inner: Socket,
}

impl UnixStream {
    /// Opens a Unix connection to the specified file path. There must be a
    /// `UnixListener` or equivalent listening on the corresponding Unix domain socket
    /// to successfully connect and return a `UnixStream`.
    pub fn connect(path: impl AsRef<Path>) -> IoResult<UnixStream> {
        let socket = Socket::new(AF_UNIX as _, SOCK_STREAM, 0)?;
        socket.connect(path.as_ref().to_path_buf())?;
        let unix_stream = UnixStream { inner: socket };
        Ok(unix_stream)
    }

    /// Returns the socket address of the remote peer of this TCP connection.
    pub fn peer_addr(&self) -> IoResult<PathBuf> {
        self.inner.peer_addr()
    }

    /// Returns the socket address of the local half of this TCP connection.
    pub fn local_addr(&self) -> IoResult<PathBuf> {
        self.inner.local_addr()
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
