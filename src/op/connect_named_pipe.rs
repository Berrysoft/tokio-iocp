use crate::op::*;
use windows_sys::Win32::System::Pipes::ConnectNamedPipe;

pub struct ConnectNamedPipe {}

impl ConnectNamedPipe {
    pub fn new() -> Self {
        Self {}
    }
}

impl IocpOperation for ConnectNamedPipe {
    type Output = ();

    type Buffer = ();

    unsafe fn operate(
        &mut self,
        handle: usize,
        overlapped_ptr: *mut OVERLAPPED,
    ) -> Poll<IoResult<()>> {
        let res = ConnectNamedPipe(handle as _, overlapped_ptr);
        win32_result(res)
    }

    fn set_buf_init(&mut self, _len: usize) {}

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Self::Output, Self::Buffer> {
        (res.map(|_| ()), ())
    }
}
