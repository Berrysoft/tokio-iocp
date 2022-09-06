use crate::op::*;
use once_cell::sync::OnceCell as OnceLock;
use std::{
    os::windows::prelude::{AsRawSocket, OwnedSocket},
    ptr::null_mut,
};
use windows_sys::Win32::Networking::WinSock::{LPFN_ACCEPTEX, LPFN_GETACCEPTEXSOCKADDRS};

static ACCEPT_EX: OnceLock<LPFN_ACCEPTEX> = OnceLock::new();
static GET_ADDRS: OnceLock<LPFN_GETACCEPTEXSOCKADDRS> = OnceLock::new();

pub struct Accept {
    accept_handle: Option<OwnedSocket>,
    addr_buffer: [u8; MAX_ADDR_SIZE * 2],
}

impl Accept {
    pub fn new(handle: OwnedSocket) -> Self {
        Self {
            accept_handle: Some(handle),
            addr_buffer: [0; MAX_ADDR_SIZE * 2],
        }
    }
}

impl IocpOperation for Accept {
    type Output = SocketAddr;
    type Buffer = OwnedSocket;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let accept_fn = ACCEPT_EX.get_or_try_init(|| {
            let fguid = guid_from_u128(0xb5367df1_cbac_11cf_95ca_00805f48a192);
            get_wsa_fn(handle, fguid)
        })?;
        let _get_addrs_fn = GET_ADDRS.get_or_try_init(|| {
            let fguid = guid_from_u128(0xb5367df2_cbac_11cf_95ca_00805f48a192);
            get_wsa_fn(handle, fguid)
        })?;
        let mut received = 0;
        let res = accept_fn.unwrap()(
            handle,
            self.accept_handle.as_ref().unwrap().as_raw_socket() as _,
            self.addr_buffer.as_ptr() as _,
            0,
            MAX_ADDR_SIZE as _,
            MAX_ADDR_SIZE as _,
            &mut received,
            overlapped_ptr,
        );
        wsa_result(res)
    }

    fn set_buf_len(&mut self, _len: usize) {}

    fn result(&mut self, _res: usize) -> BufResult<Self::Output, Self::Buffer> {
        let remote_addr = unsafe {
            let mut local_addr: *mut SOCKADDR = null_mut();
            let mut local_addr_len = 0;
            let mut remote_addr: *mut SOCKADDR = null_mut();
            let mut remote_addr_len = 0;
            (GET_ADDRS.get().unwrap().unwrap())(
                self.addr_buffer.as_ptr() as _,
                0,
                MAX_ADDR_SIZE as _,
                MAX_ADDR_SIZE as _,
                &mut local_addr,
                &mut local_addr_len,
                &mut remote_addr,
                &mut remote_addr_len,
            );
            wsa_get_addr(remote_addr, remote_addr_len as _)
        };
        (Ok(remote_addr), self.accept_handle.take().unwrap())
    }

    fn error(&mut self, err: IoError) -> BufResult<Self::Output, Self::Buffer> {
        (Err(err), self.accept_handle.take().unwrap())
    }
}
