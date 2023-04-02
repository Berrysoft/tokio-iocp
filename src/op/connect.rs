use crate::{net::*, op::*};
use once_cell::sync::OnceCell as OnceLock;
use std::ptr::null;
use windows_sys::Win32::Networking::WinSock::LPFN_CONNECTEX;

static CONNECT_EX: OnceLock<LPFN_CONNECTEX> = OnceLock::new();

pub struct Connect<A: SockAddr> {
    addr: A,
}

impl<A: SockAddr> Connect<A> {
    pub fn new(addr: A) -> Self {
        Self { addr }
    }
}

impl<A: SockAddr> IocpOperation for Connect<A> {
    type Output = ();
    type Buffer = ();

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let connect_fn = CONNECT_EX.get_or_try_init(|| {
            let fguid = GUID::from_u128(0x25a207b9_ddf3_4660_8ee9_76e58c74063e);
            get_wsa_fn(handle, fguid)
        })?;
        let mut sent = 0;
        let res = self.addr.with_native(|addr, len| {
            connect_fn.unwrap()(handle, addr, len, null(), 0, &mut sent, overlapped_ptr)
        });
        win32_result(res)
    }

    fn set_buf_init(&mut self, _len: usize) {}

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Self::Output, Self::Buffer> {
        (res.map(|_| ()), ())
    }
}
