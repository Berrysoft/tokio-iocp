use crate::{fs::File, *};
use std::{fs::OpenOptions as StdOpenOptions, os::windows::prelude::OpenOptionsExt, path::Path};
use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED;

#[derive(Debug, Clone)]
pub struct OpenOptions(StdOpenOptions);

impl OpenOptions {
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        let mut options = StdOpenOptions::new();
        options.custom_flags(FILE_FLAG_OVERLAPPED);
        Self(options)
    }

    pub fn read(&mut self, read: bool) -> &mut Self {
        self.0.read(read);
        self
    }

    pub fn write(&mut self, write: bool) -> &mut Self {
        self.0.write(write);
        self
    }

    pub fn append(&mut self, append: bool) -> &mut Self {
        self.0.append(append);
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.0.truncate(truncate);
        self
    }

    pub fn create(&mut self, create: bool) -> &mut Self {
        self.0.create(create);
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.0.create_new(create_new);
        self
    }

    pub fn open(&self, path: impl AsRef<Path>) -> IoResult<File> {
        File::from_handle(self.0.open(path)?.into())
    }
}
