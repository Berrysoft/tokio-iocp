mod read_at;
pub use read_at::*;
mod write_at;
pub use write_at::*;

use crate::{io_port::IO_PORT, *};
use std::{
    fs::OpenOptions,
    ops::Deref,
    os::windows::fs::OpenOptionsExt,
    os::windows::prelude::{AsHandle, OwnedHandle},
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

    pub fn read_at(&self, buffer: Vec<u8>, pos: usize) -> FileAsyncRead {
        FileAsyncRead::new(self.as_handle(), pos as _, buffer)
    }

    pub fn write_at(&self, buffer: Vec<u8>, pos: usize) -> FileAsyncWrite {
        FileAsyncWrite::new(self.as_handle(), pos as _, buffer)
    }
}

impl Deref for File {
    type Target = OwnedHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
