mod io_at;
pub use io_at::*;

use crate::{buf::*, io_port::IO_PORT, op::fs::*, *};
use std::{
    fs::OpenOptions,
    ops::Deref,
    os::windows::fs::OpenOptionsExt,
    os::windows::io::{AsHandle, OwnedHandle},
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
        IO_PORT.attach(self)
    }

    pub fn read_at<T: IoBufMut>(&self, buffer: T, pos: usize) -> FileAsyncIoAt<Read<T>> {
        FileAsyncIoAt::new(self.as_handle(), pos as _, buffer, Read::default())
    }

    pub fn write_at<T: IoBuf>(&self, buffer: T, pos: usize) -> FileAsyncIoAt<Write<T>> {
        FileAsyncIoAt::new(self.as_handle(), pos as _, buffer, Write::default())
    }
}

impl Deref for File {
    type Target = OwnedHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
