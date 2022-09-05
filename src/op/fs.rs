use crate::{buf::*, op::*, *};
use windows_sys::Win32::{
    Foundation::{ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE},
    Storage::FileSystem::{ReadFile, WriteFile},
    System::IO::OVERLAPPED,
};

unsafe fn retrieve_result(res: i32) -> IoResult<()> {
    if res == 0 {
        let error = GetLastError();
        match error {
            ERROR_IO_PENDING | ERROR_IO_INCOMPLETE | ERROR_HANDLE_EOF => Ok(()),
            _ => Err(IoError::from_raw_os_error(error as _)),
        }
    } else {
        Ok(())
    }
}

pub struct ReadAt<T: IoBufMut> {
    buffer: T,
    pos: usize,
}

impl<T: IoBufMut> ReadAt<T> {
    pub fn new(buffer: T, pos: usize) -> Self {
        Self { buffer, pos }
    }
}

impl<T: IoBufMut> IocpOperation for ReadAt<T> {
    type Output = usize;
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        if let Some(overlapped) = overlapped_ptr.as_mut() {
            overlapped.Anonymous.Anonymous.Offset = self.pos as _;
        }
        let mut read = 0;
        let res = ReadFile(
            handle as _,
            self.buffer.as_buf_mut_ptr() as _,
            self.buffer.buf_len() as _,
            &mut read,
            overlapped_ptr,
        );
        retrieve_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take())
    }
}

pub struct WriteAt<T: IoBuf> {
    buffer: T,
    pos: usize,
}

impl<T: IoBuf> WriteAt<T> {
    pub fn new(buffer: T, pos: usize) -> Self {
        Self { buffer, pos }
    }
}

impl<T: IoBuf> IocpOperation for WriteAt<T> {
    type Output = usize;
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        if let Some(overlapped) = overlapped_ptr.as_mut() {
            overlapped.Anonymous.Anonymous.Offset = self.pos as _;
        }
        let mut written = 0;
        let res = WriteFile(
            handle as _,
            self.buffer.as_buf_ptr() as _,
            self.buffer.buf_len() as _,
            &mut written,
            overlapped_ptr,
        );
        retrieve_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take())
    }
}
