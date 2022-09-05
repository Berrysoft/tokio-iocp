use crate::{buf::*, op::*, *};
use windows_sys::Win32::{Storage::FileSystem::WriteFile, System::IO::OVERLAPPED};

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
        win32_result(res)
    }

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take())
    }
}
