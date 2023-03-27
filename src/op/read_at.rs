use crate::op::*;
use windows_sys::Win32::Storage::FileSystem::ReadFile;

pub struct ReadAt<T: IoBufMut> {
    buffer: BufWrapper<T>,
    pos: usize,
}

impl<T: IoBufMut> ReadAt<T> {
    pub fn new(buffer: T, pos: usize) -> Self {
        Self {
            buffer: BufWrapper::new(buffer),
            pos,
        }
    }
}

impl<T: IoBufMut> IocpOperation for ReadAt<T> {
    type Output = usize;
    type Buffer = T;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        if let Some(overlapped) = overlapped_ptr.as_mut() {
            overlapped.Anonymous.Anonymous.Offset = self.pos as _;
        }
        let res = self.buffer.with_buf_mut(|ptr, len| {
            let mut read = 0;
            ReadFile(handle as _, ptr as _, len as _, &mut read, overlapped_ptr)
        });
        win32_result(res)
    }

    fn set_buf_init(&mut self, len: usize) {
        self.buffer.set_init(len)
    }

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Self::Output, Self::Buffer> {
        (res, self.buffer.take_buf())
    }
}
