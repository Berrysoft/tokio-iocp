use crate::{
    buf::*,
    io_port::{file::*, IO_PORT},
    op::{read_at::*, write_at::*},
    *,
};
use std::{
    fs::OpenOptions,
    ops::Deref,
    os::windows::io::{AsHandle, OwnedHandle},
    os::windows::{fs::OpenOptionsExt, prelude::AsRawHandle},
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED;

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

    pub fn create(path: impl AsRef<Path>) -> IoResult<Self> {
        let file = Self {
            handle: OpenOptions::new()
                .create(true)
                .write(true)
                .custom_flags(FILE_FLAG_OVERLAPPED)
                .open(path)?
                .into(),
        };
        file.attach()?;
        Ok(file)
    }

    fn attach(&self) -> IoResult<()> {
        IO_PORT.attach(self.handle.as_raw_handle() as _)
    }

    pub async fn read_at<T: IoBufMut>(&self, buffer: T, pos: usize) -> BufResult<usize, T> {
        FileFuture::new(self.as_handle(), ReadAt::new(buffer, pos)).await
    }

    pub async fn write_at<T: IoBuf>(&self, buffer: T, pos: usize) -> BufResult<usize, T> {
        FileFuture::new(self.as_handle(), WriteAt::new(buffer, pos)).await
    }
}

impl Deref for File {
    type Target = OwnedHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
