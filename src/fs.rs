use crate::{
    io_port::{OverlappedWaker, IO_PORT},
    *,
};
use std::{
    fs::OpenOptions,
    future::Future,
    ops::Deref,
    os::windows::fs::OpenOptionsExt,
    os::windows::prelude::{AsHandle, AsRawHandle, BorrowedHandle, OwnedHandle},
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};
use windows_sys::Win32::{
    Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING},
    Storage::FileSystem::{ReadFile, FILE_FLAG_OVERLAPPED},
    System::IO::GetOverlappedResult,
};

#[derive(Debug)]
pub struct File {
    handle: OwnedHandle,
}

impl File {
    pub fn open(path: impl AsRef<Path>) -> IoResult<Self> {
        let file = Self {
            handle: OpenOptions::new()
                .read(true)
                .custom_flags(FILE_FLAG_OVERLAPPED)
                .open(path)?
                .into(),
        };
        file.attach()?;
        Ok(file)
    }

    fn attach(&self) -> IoResult<()> {
        IO_PORT.attach(self)
    }

    pub fn read_at(&self, buffer: Vec<u8>, pos: usize) -> FileAsyncRead {
        FileAsyncRead {
            handle: self.as_handle(),
            pos: pos as _,
            buffer,
            overlapped_ptr: 0,
        }
    }
}

impl Deref for File {
    type Target = OwnedHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

pub struct FileAsyncRead<'a> {
    handle: BorrowedHandle<'a>,
    pos: u32,
    buffer: Vec<u8>,
    overlapped_ptr: usize,
}

impl FileAsyncRead<'_> {
    fn result(mut self: Pin<&mut Self>, res: IoResult<usize>) -> (IoResult<usize>, Vec<u8>) {
        (res, std::mem::take(&mut self.buffer))
    }
}

impl Future for FileAsyncRead<'_> {
    type Output = (IoResult<usize>, Vec<u8>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.buffer.is_empty() {
            Poll::Ready(self.result(Ok(0)))
        } else if self.overlapped_ptr == 0 {
            let mut overlapped = Box::new(OverlappedWaker::new());
            overlapped.overlapped.Anonymous.Anonymous.Offset = self.pos;
            overlapped.set_waker(cx.waker().clone());
            let overlapped_ptr = overlapped.leak();
            let mut read = 0;
            let res = unsafe {
                ReadFile(
                    self.handle.as_raw_handle() as _,
                    self.buffer.as_mut_ptr() as _,
                    self.buffer.len() as _,
                    &mut read,
                    overlapped_ptr as _,
                )
            };
            self.overlapped_ptr = overlapped_ptr as usize;
            if res == 0 {
                let error = unsafe { GetLastError() };
                match error {
                    ERROR_IO_PENDING => Poll::Pending,
                    _ => Poll::Ready(self.result(Err(IoError::from_raw_os_error(error as _)))),
                }
            } else {
                Poll::Ready(self.result(Ok(read as _)))
            }
        } else {
            let mut transferred = 0;
            let res = unsafe {
                GetOverlappedResult(
                    self.handle.as_raw_handle() as _,
                    self.overlapped_ptr as _,
                    &mut transferred,
                    0,
                )
            };
            if res == 0 {
                let error = unsafe { GetLastError() };
                match error {
                    ERROR_IO_INCOMPLETE => Poll::Pending,
                    ERROR_HANDLE_EOF => Poll::Ready(self.result(Ok(0))),
                    _ => Poll::Ready(self.result(Err(IoError::from_raw_os_error(error as _)))),
                }
            } else {
                self.overlapped_ptr = 0;
                self.pos += transferred;
                Poll::Ready(self.result(Ok(transferred as _)))
            }
        }
    }
}
