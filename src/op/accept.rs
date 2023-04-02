use crate::{net::*, op::*};
use aligned_array::{Aligned, A4};
use once_cell::sync::OnceCell as OnceLock;
use std::{
    marker::PhantomData,
    os::windows::prelude::{AsRawSocket, OwnedSocket},
    ptr::{null_mut, NonNull},
};
use windows_sys::Win32::Networking::WinSock::{
    LPFN_ACCEPTEX, LPFN_GETACCEPTEXSOCKADDRS, WSAID_ACCEPTEX, WSAID_GETACCEPTEXSOCKADDRS,
};

static ACCEPT_EX: OnceLock<LPFN_ACCEPTEX> = OnceLock::new();
static GET_ADDRS: OnceLock<LPFN_GETACCEPTEXSOCKADDRS> = OnceLock::new();

pub struct Accept<A: SockAddr> {
    accept_handle: Option<OwnedSocket>,
    addr_buffer: Aligned<A4, [u8; MAX_ADDR_SIZE * 2]>,
    _marker: PhantomData<A>,
}

impl<A: SockAddr> Accept<A> {
    pub fn new(handle: OwnedSocket) -> Self {
        Self {
            accept_handle: Some(handle),
            addr_buffer: Aligned([0; MAX_ADDR_SIZE * 2]),
            _marker: PhantomData,
        }
    }
}

impl<A: SockAddr> IocpOperation for Accept<A> {
    type Output = A;
    type Buffer = OwnedSocket;

    unsafe fn operate(&mut self, handle: usize, overlapped_ptr: *mut OVERLAPPED) -> IoResult<()> {
        let accept_fn = ACCEPT_EX.get_or_try_init(|| get_wsa_fn(handle, WSAID_ACCEPTEX))?;
        let _get_addrs_fn =
            GET_ADDRS.get_or_try_init(|| get_wsa_fn(handle, WSAID_GETACCEPTEXSOCKADDRS))?;
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
        win32_result(res)
    }

    fn set_buf_init(&mut self, _len: usize) {}

    fn result(&mut self, res: IoResult<usize>) -> BufResult<Self::Output, Self::Buffer> {
        let out = res.map(|_| unsafe {
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
            A::try_from_native(NonNull::new(remote_addr).unwrap(), remote_addr_len).unwrap()
        });
        (out, self.accept_handle.take().unwrap())
    }
}
