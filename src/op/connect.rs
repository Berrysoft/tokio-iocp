use crate::op::*;
use std::{ptr::null, sync::OnceLock};
use windows_sys::Win32::Networking::WinSock::LPFN_CONNECTEX;

static CONNECT_EX: OnceLock<LPFN_CONNECTEX> = OnceLock::new();

pub struct Connect {
    addr: SocketAddr,
}

impl Connect {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

impl IocpOperation for Connect {
    type Output = ();
    type Buffer = ();

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let connect_fn = CONNECT_EX.get_or_try_init(|| {
            let fguid = guid_from_u128(0x25a207b9_ddf3_4660_8ee9_76e58c74063e);
            get_wsa_fn(handle, fguid)
        })?;
        let mut sent = 0;
        let res = wsa_exact_addr(self.addr, |addr, len| {
            connect_fn.unwrap()(handle, addr, len, null(), 0, &mut sent, overlapped_ptr)
        });
        wsa_result(res)
    }

    fn set_buf_len(&mut self, _len: usize) {}

    fn result(&mut self, _res: usize) -> BufResult<Self::Output, Self::Buffer> {
        (Ok(()), ())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), ())
    }
}
