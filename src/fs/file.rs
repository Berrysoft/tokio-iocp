use crate::{buf::*, fs::OpenOptions, io_port::IO_PORT, op, *};
use std::{
    os::windows::prelude::{
        AsHandle, AsRawHandle, BorrowedHandle, IntoRawHandle, OwnedHandle, RawHandle,
    },
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::FlushFileBuffers;

/// A reference to an open file on the filesystem.
///
/// An instance of a `File` can be read and/or written depending on what options
/// it was opened with. The `File` type provides **positional** read and write
/// operations. The file does not maintain an internal cursor. The caller is
/// required to specify an offset when issuing an operation.
///
/// # Examples
///
/// Creates a new file and write data to it:
///
/// ```
/// use tempfile::NamedTempFile;
/// use tokio_iocp::{fs::File, IoResult};
///
/// fn main() -> IoResult<()> {
///     tokio_iocp::start(async {
///         // Open a file
///         let file = File::create(NamedTempFile::new()?)?;
///
///         // Write some data
///         let (res, buf) = file.write_at("hello world", 0).await;
///         let n = res?;
///
///         println!("wrote {} bytes", n);
///
///         Ok(())
///     })
/// }
/// ```
#[derive(Debug)]
pub struct File {
    handle: OwnedHandle,
}

impl File {
    /// Attempts to open a file in read-only mode.
    ///
    /// See the [`OpenOptions::open`] method for more details.
    pub fn open(path: impl AsRef<Path>) -> IoResult<Self> {
        OpenOptions::new().read(true).open(path)
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist,
    /// and will truncate it if it does.
    ///
    /// See the [`OpenOptions::open`] function for more details.
    pub fn create(path: impl AsRef<Path>) -> IoResult<Self> {
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
    }

    pub(crate) fn from_handle(handle: OwnedHandle) -> IoResult<Self> {
        let file = Self { handle };
        file.attach()?;
        Ok(file)
    }

    /// Create an [`OpenOptions`].
    /// It is the same as [`OpenOptions::new`].
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }

    fn attach(&self) -> IoResult<()> {
        IO_PORT.with(|port| port.attach(self.handle.as_raw_handle() as _))
    }

    /// Read some bytes at the specified offset from the file into the specified
    /// buffer, returning how many bytes were read.
    ///
    /// # Return
    ///
    /// The method returns the operation result and the same buffer value passed
    /// as an argument.
    ///
    /// If the method returns [`Ok(n)`], then the read was successful. A nonzero
    /// `n` value indicates that the buffer has been filled with `n` bytes of
    /// data from the file. If `n` is `0`, then one of the following happened:
    ///
    /// 1. The specified offset is the end of the file.
    /// 2. The buffer specified was 0 bytes in capacity.
    ///
    /// It is not an error if the returned value `n` is smaller than the buffer
    /// size, even when the file contains enough data to fill the buffer.
    ///
    /// # Errors
    ///
    /// If this function encounters any form of I/O or other error, an error
    /// variant will be returned. The buffer is returned on error.
    pub async fn read_at<T: IoBufMut>(&self, buffer: T, pos: usize) -> BufResult<usize, T> {
        let (res, mut buffer) = op::read_at(self.as_handle(), buffer, pos).await;
        if let Ok(init) = res {
            buffer.set_init(init);
        }
        (res, buffer.into_inner())
    }

    /// Write a buffer into this file at the specified offset, returning how
    /// many bytes were written.
    ///
    /// This function will attempt to write the entire contents of `buf`, but
    /// the entire write may not succeed, or the write may also generate an
    /// error. The bytes will be written starting at the specified offset.
    ///
    /// # Return
    ///
    /// The method returns the operation result and the same buffer value passed
    /// in as an argument. A return value of `0` typically means that the
    /// underlying file is no longer able to accept bytes and will likely not be
    /// able to in the future as well, or that the buffer provided is empty.
    ///
    /// # Errors
    ///
    /// Each call to `write_at` may generate an I/O error indicating that the
    /// operation could not be completed. If an error is returned then no bytes
    /// in the buffer were written to this writer.
    ///
    /// It is **not** considered an error if the entire buffer could not be
    /// written to this writer.
    pub async fn write_at<T: IoBuf>(&self, buffer: T, pos: usize) -> BufResult<usize, T> {
        let (res, buffer) = op::write_at(self.as_handle(), buffer, pos).await;
        (res, buffer.into_inner())
    }

    /// Attempts to flush write buffers to disk.
    ///
    /// This function will error if the file doesn't have write permission.
    pub fn flush(&self) -> IoResult<()> {
        let res = unsafe { FlushFileBuffers(self.as_raw_handle() as _) };
        if res == 0 {
            Err(IoError::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl AsRawHandle for File {
    fn as_raw_handle(&self) -> RawHandle {
        self.handle.as_raw_handle()
    }
}

impl IntoRawHandle for File {
    fn into_raw_handle(self) -> RawHandle {
        self.handle.into_raw_handle()
    }
}

impl AsHandle for File {
    fn as_handle(&self) -> BorrowedHandle {
        self.handle.as_handle()
    }
}
