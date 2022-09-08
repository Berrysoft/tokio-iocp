use crate::op::*;
use windows_sys::Win32::Storage::FileSystem::WriteFile;

pub struct WriteAt<T: IoBuf> {
    buffer: BufWrapper<T>,
    pos: usize,
}

impl<T: IoBuf> WriteAt<T> {
    pub fn new(buffer: T, pos: usize) -> Self {
        Self {
            buffer: BufWrapper::new(buffer),
            pos,
        }
    }
}

impl<T: IoBuf> IocpOperation for WriteAt<T> {
    type Output = usize;
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        if let Some(overlapped) = overlapped_ptr.as_mut() {
            overlapped.Anonymous.Anonymous.Offset = self.pos as _;
        }
        let res = self.buffer.with_buf(|ptr, len| {
            let mut written = 0;
            WriteFile(
                handle as _,
                ptr as _,
                len as _,
                &mut written,
                overlapped_ptr,
            )
        });
        win32_result(res)
    }

    fn set_buf_len(&mut self, _len: usize) {}

    fn result(&mut self, res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(res), self.buffer.take_buf())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.buffer.take_buf())
    }
}
