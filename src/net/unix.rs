use crate::{
    buf::*,
    net::{Socket, *},
    *,
};
use std::{net::Shutdown, path::Path, str::FromStr};
use windows_sys::Win32::Networking::WinSock::{AF_UNIX, IPPROTO_HOPOPTS, SOCK_STREAM};

const UNIX_MAX_PATH: usize = 108;

/// An address associated with a Unix socket.
///
/// Socket file path should not be longer than 107 bytes.
///
/// # Examples
///
/// ```
/// use tokio_iocp::net::UnixSocketAddr;
/// use std::path::Path;
///
/// assert!(UnixSocketAddr::unnamed().is_unnamed());
/// let addr = "C:\\path.sock".parse::<UnixSocketAddr>().unwrap();
/// assert_eq!(addr.as_pathname(), Some(Path::new("C:\\path.sock")));
/// ```
///
/// # Notes on Windows
///
/// Windows only provides the same behavior as UNIX for `pathname` address.
/// Connecting to `abstract` address has not been supported, and the length
/// of non-`pathname` address may be wrong.
#[derive(Debug, Clone, Copy)]
pub struct UnixSocketAddr {
    pub(crate) path: [u8; UNIX_MAX_PATH],
    pub(crate) len: usize,
}

impl UnixSocketAddr {
    /// Creates an unnamed address.
    pub fn unnamed() -> Self {
        Self {
            path: [0u8; UNIX_MAX_PATH],
            len: 0,
        }
    }

    /// Creates an anddress from file path.
    /// The length of file path should not be longer than 107 bytes.
    pub fn from_pathname(path: impl AsRef<Path>) -> IoResult<Self> {
        let path = path.as_ref().as_os_str().to_string_lossy();
        let bytes = path.as_bytes();
        if bytes.contains(&0) {
            return Err(IoError::new(
                std::io::ErrorKind::InvalidInput,
                "paths must not contain interior null bytes",
            ));
        }
        if bytes.len() >= UNIX_MAX_PATH - 1 {
            return Err(IoError::new(
                std::io::ErrorKind::InvalidInput,
                "path must be shorter than UNIX_MAX_PATH",
            ));
        }
        let mut path = [0u8; UNIX_MAX_PATH];
        path[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            path,
            len: bytes.len() + 1,
        })
    }

    /// Creates an anddress with abstract namespace.
    /// The length of namespace should not be longer than 107 bytes.
    pub fn from_abstract_namespace(namespace: &[u8]) -> IoResult<Self> {
        if namespace.len() >= UNIX_MAX_PATH - 1 {
            return Err(IoError::new(
                std::io::ErrorKind::InvalidInput,
                "namespace must be shorter than UNIX_MAX_PATH",
            ));
        }
        let mut path = [0u8; UNIX_MAX_PATH];
        path[1..namespace.len() + 1].copy_from_slice(namespace);
        Ok(Self {
            path,
            len: namespace.len() + 1,
        })
    }

    fn address(&self) -> UnixAddressKind<'_> {
        if self.len == 0 {
            UnixAddressKind::Unnamed
        } else if self.path[0] == 0 {
            UnixAddressKind::Abstract(&self.path[1..self.len])
        } else {
            UnixAddressKind::Pathname(
                unsafe { std::str::from_utf8_unchecked(&self.path[0..self.len - 1]) }.as_ref(),
            )
        }
    }

    /// Returns `true` if the address is unnamed.
    #[must_use]
    pub fn is_unnamed(&self) -> bool {
        matches!(self.address(), UnixAddressKind::Unnamed)
    }

    /// Returns the contents of this address if it is a `pathname` address.
    #[must_use]
    pub fn as_pathname(&self) -> Option<&Path> {
        if let UnixAddressKind::Pathname(path) = self.address() {
            Some(path)
        } else {
            None
        }
    }

    /// Returns the contents of this address if it is an abstract namespace
    /// without the leading null byte.
    #[must_use]
    pub fn as_abstract_namespace(&self) -> Option<&[u8]> {
        if let UnixAddressKind::Abstract(name) = self.address() {
            Some(name)
        } else {
            None
        }
    }
}

impl std::fmt::Display for UnixSocketAddr {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.address() {
            UnixAddressKind::Unnamed => write!(fmt, "(unnamed)"),
            UnixAddressKind::Abstract(name) => write!(fmt, "{} (abstract)", unsafe {
                std::str::from_utf8_unchecked(name)
            }),
            UnixAddressKind::Pathname(path) => write!(fmt, "{} (pathname)", path.display()),
        }
    }
}

impl FromStr for UnixSocketAddr {
    type Err = IoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_pathname(s)
    }
}

#[derive(Debug)]
enum UnixAddressKind<'a> {
    Unnamed,
    Pathname(&'a Path),
    Abstract(&'a [u8]),
}

/// A Unix socket server, listening for connections.
///
/// You can accept a new connection by using the [`UnixListener::accept`] method.
///
/// # Examples
///
/// ```
/// use tokio_iocp::net::{UnixListener, UnixStream};
/// use tempfile::tempdir;
///
/// let dir = tempdir().unwrap();
/// let sock_file = dir.path().join("unix-server.sock");
///
/// tokio_iocp::start(async move {
///     let listener = UnixListener::bind(&sock_file).unwrap();
///
///     let tx = UnixStream::connect(&sock_file).unwrap();
///     let (rx, _) = listener.accept().await.unwrap();
///
///     tx.send("test").await.0.unwrap();
///
///     let (res, buf) = rx.recv(Vec::with_capacity(4)).await;
///     res.unwrap();
///
///     assert_eq!(buf, b"test");
/// });
/// ```
pub struct UnixListener {
    inner: Socket,
}

impl UnixListener {
    /// Creates a new [`UnixListener`], which will be bound to the specified file path.
    /// The file path cannot yet exist, and will be cleaned up upon dropping [`UnixListener`]
    pub fn bind(path: impl AsRef<Path>) -> IoResult<Self> {
        Self::bind_addr(UnixSocketAddr::from_pathname(path)?)
    }

    /// Creates a new [`UnixListener`] with [`UnixSocketAddr`], which will be bound to the specified file path.
    /// The file path cannot yet exist, and will be cleaned up upon dropping [`UnixListener`]
    pub fn bind_addr(addr: UnixSocketAddr) -> IoResult<Self> {
        let socket = Socket::bind(addr, SOCK_STREAM, 0)?;
        socket.listen(1024)?;
        Ok(UnixListener { inner: socket })
    }

    /// Accepts a new incoming connection from this listener.
    ///
    /// This function will yield once a new Unix domain socket connection
    /// is established. When established, the corresponding [`UnixStream`] and
    /// will be returned.
    pub async fn accept(&self) -> IoResult<(UnixStream, UnixSocketAddr)> {
        let (socket, addr) = self
            .inner
            .accept::<UnixSocketAddr>(SOCK_STREAM, IPPROTO_HOPOPTS)
            .await?;
        let stream = UnixStream { inner: socket };
        Ok((stream, addr))
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_iocp::net::UnixListener;
    /// use std::path::Path;
    /// use tempfile::tempdir;
    ///
    /// let dir = tempdir().unwrap();
    /// let sock_file = dir.path().join("unix-server.sock");
    /// let listener = UnixListener::bind(&sock_file).unwrap();
    ///
    /// let addr = listener.local_addr().expect("Couldn't get local address");
    /// assert_eq!(addr.as_pathname(), Some(Path::new(&sock_file)));
    /// ```
    pub fn local_addr(&self) -> IoResult<UnixSocketAddr> {
        self.inner.local_addr()
    }
}

impl_socket!(UnixListener, inner);

/// A Unix stream between two local sockets on Windows & WSL.
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
    /// [`UnixListener`] or equivalent listening on the corresponding Unix domain socket
    /// to successfully connect and return a `UnixStream`.
    pub fn connect(path: impl AsRef<Path>) -> IoResult<Self> {
        Self::connect_addr(UnixSocketAddr::from_pathname(path)?)
    }

    /// Opens a Unix connection to the specified address. There must be a
    /// [`UnixListener`] or equivalent listening on the corresponding Unix domain socket
    /// to successfully connect and return a `UnixStream`.
    pub fn connect_addr(addr: UnixSocketAddr) -> IoResult<Self> {
        let socket = Socket::new(AF_UNIX, SOCK_STREAM, IPPROTO_HOPOPTS)?;
        socket.connect(addr)?;
        let unix_stream = UnixStream { inner: socket };
        Ok(unix_stream)
    }

    /// Returns the socket path of the remote peer of this connection.
    pub fn peer_addr(&self) -> IoResult<UnixSocketAddr> {
        self.inner.peer_addr()
    }

    /// Returns the socket path of the local half of this connection.
    pub fn local_addr(&self) -> IoResult<UnixSocketAddr> {
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

impl_socket!(UnixStream, inner);
