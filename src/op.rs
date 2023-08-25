use crate::{
    buf::*,
    io_port::IocpFuture,
    net::{SockAddr, MAX_ADDR_SIZE},
    *,
};
use aligned_array::{Aligned, A4};
use once_cell::sync::OnceCell as OnceLock;
use std::{
    os::windows::prelude::{AsRawSocket, BorrowedHandle, BorrowedSocket},
    ptr::{null, null_mut, NonNull},
    task::Poll,
};
use windows_sys::{
    core::GUID,
    Win32::{
        Foundation::{
            GetLastError, ERROR_HANDLE_EOF, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_NO_DATA,
            ERROR_PIPE_CONNECTED,
        },
        Networking::WinSock::{
            WSAIoctl, WSARecv, WSARecvFrom, WSASend, WSASendTo, LPFN_ACCEPTEX, LPFN_CONNECTEX,
            LPFN_GETACCEPTEXSOCKADDRS, SIO_GET_EXTENSION_FUNCTION_POINTER, SOCKADDR,
            WSAID_ACCEPTEX, WSAID_CONNECTEX, WSAID_GETACCEPTEXSOCKADDRS,
        },
        Storage::FileSystem::{ReadFile, WriteFile},
        System::Pipes::ConnectNamedPipe,
    },
};

pub unsafe fn win32_result(res: i32) -> Poll<IoResult<()>> {
    if res == 0 {
        let error = GetLastError();
        match error {
            ERROR_IO_PENDING => Poll::Pending,
            0 | ERROR_IO_INCOMPLETE | ERROR_HANDLE_EOF | ERROR_PIPE_CONNECTED | ERROR_NO_DATA => {
                Poll::Ready(Ok(()))
            }
            _ => Poll::Ready(Err(IoError::from_raw_os_error(error as _))),
        }
    } else {
        Poll::Ready(Ok(()))
    }
}

unsafe fn get_wsa_fn<F>(handle: usize, fguid: GUID) -> IoResult<Option<F>> {
    let mut fptr = None;
    let mut returned = 0;
    let res = WSAIoctl(
        handle,
        SIO_GET_EXTENSION_FUNCTION_POINTER,
        std::ptr::addr_of!(fguid).cast(),
        std::mem::size_of_val(&fguid) as _,
        std::ptr::addr_of_mut!(fptr).cast(),
        std::mem::size_of::<F>() as _,
        &mut returned,
        null_mut(),
        None,
    );
    if res == 0 {
        Ok(fptr)
    } else {
        Err(IoError::last_os_error())
    }
}

pub fn read_at<T: IoBufMut>(
    handle: BorrowedHandle,
    buffer: T,
    pos: usize,
) -> IocpFuture<BufWrapper<T>> {
    IocpFuture::new(
        handle,
        BufWrapper::new(buffer),
        |handle, overlapped_ptr, buffer| unsafe {
            if let Some(overlapped) = overlapped_ptr.as_mut() {
                overlapped.Anonymous.Anonymous.Offset = (pos & 0xFFFFFFFF) as _;
                overlapped.Anonymous.Anonymous.OffsetHigh = (pos >> 32) as _;
            }
            let res = buffer.with_buf_mut(|ptr, len| {
                let mut read = 0;
                ReadFile(handle as _, ptr as _, len as _, &mut read, overlapped_ptr)
            });
            win32_result(res)
        },
    )
}

pub fn write_at<T: IoBuf>(
    handle: BorrowedHandle,
    buffer: T,
    pos: usize,
) -> IocpFuture<BufWrapper<T>> {
    IocpFuture::new(
        handle,
        BufWrapper::new(buffer),
        |handle, overlapped_ptr, buffer| unsafe {
            if let Some(overlapped) = overlapped_ptr.as_mut() {
                overlapped.Anonymous.Anonymous.Offset = (pos & 0xFFFFFFFF) as _;
                overlapped.Anonymous.Anonymous.OffsetHigh = (pos >> 32) as _;
            }
            let res = buffer.with_buf(|ptr, len| {
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
        },
    )
}

static ACCEPT_EX: OnceLock<LPFN_ACCEPTEX> = OnceLock::new();

pub type AcceptBuffer = Aligned<A4, [u8; MAX_ADDR_SIZE * 2]>;

pub fn accept<'a>(
    handle: BorrowedSocket<'a>,
    accept_handle: BorrowedSocket,
) -> IocpFuture<'a, AcceptBuffer> {
    IocpFuture::new(
        handle,
        Aligned([0; MAX_ADDR_SIZE * 2]),
        |handle, overlapped_ptr, buffer| unsafe {
            let accept_fn = ACCEPT_EX.get_or_try_init(|| get_wsa_fn(handle, WSAID_ACCEPTEX))?;
            let mut received = 0;
            let res = accept_fn.unwrap()(
                handle,
                accept_handle.as_raw_socket() as _,
                buffer.as_ptr() as _,
                0,
                MAX_ADDR_SIZE as _,
                MAX_ADDR_SIZE as _,
                &mut received,
                overlapped_ptr,
            );
            win32_result(res)
        },
    )
}

static GET_ADDRS: OnceLock<LPFN_GETACCEPTEXSOCKADDRS> = OnceLock::new();

pub fn accept_result<A: SockAddr>(
    handle: BorrowedSocket,
    addr_buffer: &AcceptBuffer,
) -> IoResult<A> {
    let get_addrs_fn = GET_ADDRS.get_or_try_init(|| unsafe {
        get_wsa_fn(handle.as_raw_socket() as _, WSAID_GETACCEPTEXSOCKADDRS)
    })?;
    let mut local_addr: *mut SOCKADDR = null_mut();
    let mut local_addr_len = 0;
    let mut remote_addr: *mut SOCKADDR = null_mut();
    let mut remote_addr_len = 0;
    unsafe {
        (get_addrs_fn.unwrap())(
            addr_buffer.as_ptr() as _,
            0,
            MAX_ADDR_SIZE as _,
            MAX_ADDR_SIZE as _,
            &mut local_addr,
            &mut local_addr_len,
            &mut remote_addr,
            &mut remote_addr_len,
        );
        Ok(A::try_from_native(NonNull::new(remote_addr).unwrap(), remote_addr_len).unwrap())
    }
}

static CONNECT_EX: OnceLock<LPFN_CONNECTEX> = OnceLock::new();

pub fn connect<A: SockAddr>(handle: BorrowedSocket, addr: A) -> IocpFuture<A> {
    IocpFuture::new(handle, addr, |handle, overlapped_ptr, addr| unsafe {
        let connect_fn = CONNECT_EX.get_or_try_init(|| get_wsa_fn(handle, WSAID_CONNECTEX))?;
        let mut sent = 0;
        let res = addr.with_native(|addr, len| {
            connect_fn.unwrap()(handle, addr, len, null(), 0, &mut sent, overlapped_ptr)
        });
        win32_result(res)
    })
}

pub fn recv<T: WithWsaBufMut>(handle: BorrowedSocket, buffer: T::Buffer) -> IocpFuture<T> {
    IocpFuture::new(
        handle,
        T::new(buffer),
        |handle, overlapped_ptr, buffer| unsafe {
            let res = buffer.with_wsa_buf_mut(|ptr, len| {
                let mut flags = 0;
                let mut received = 0;
                WSARecv(
                    handle,
                    ptr,
                    len as _,
                    &mut received,
                    &mut flags,
                    overlapped_ptr,
                    None,
                )
            });
            win32_result(res)
        },
    )
}

pub fn send<T: WithWsaBuf>(handle: BorrowedSocket, buffer: T::Buffer) -> IocpFuture<T> {
    IocpFuture::new(
        handle,
        T::new(buffer),
        |handle, overlapped_ptr, buffer| unsafe {
            let res = buffer.with_wsa_buf(|ptr, len| {
                let mut sent = 0;
                WSASend(handle, ptr, len as _, &mut sent, 0, overlapped_ptr, None)
            });
            win32_result(res)
        },
    )
}

pub type RecvFromBuffer = Aligned<A4, [u8; MAX_ADDR_SIZE]>;

pub fn recv_from<T: WithWsaBufMut>(
    handle: BorrowedSocket,
    buffer: T::Buffer,
) -> IocpFuture<(T, RecvFromBuffer, i32)> {
    IocpFuture::new(
        handle,
        (
            T::new(buffer),
            Aligned([0; MAX_ADDR_SIZE]),
            MAX_ADDR_SIZE as _,
        ),
        |handle, overlapped_ptr, (buffer, addr_buffer, addr_size)| unsafe {
            let res = buffer.with_wsa_buf_mut(|ptr, len| {
                let mut flags = 0;
                let mut received = 0;
                WSARecvFrom(
                    handle,
                    ptr,
                    len as _,
                    &mut received,
                    &mut flags,
                    addr_buffer.as_mut_ptr() as _,
                    addr_size,
                    overlapped_ptr,
                    None,
                )
            });
            win32_result(res)
        },
    )
}

pub fn recv_from_addr<A: SockAddr>(addr_buffer: &RecvFromBuffer, addr_size: i32) -> A {
    match unsafe {
        A::try_from_native(
            NonNull::new_unchecked(addr_buffer.as_ptr() as _),
            addr_size as _,
        )
    } {
        Some(a) => a,
        None => panic!("{:?}", addr_buffer),
    }
}

pub fn send_to<T: WithWsaBuf>(
    handle: BorrowedSocket,
    buffer: T::Buffer,
    addr: impl SockAddr,
) -> IocpFuture<T> {
    IocpFuture::new(
        handle,
        T::new(buffer),
        |handle, overlapped_ptr, buffer| unsafe {
            let res = buffer.with_wsa_buf(|ptr, len| {
                let mut sent = 0;
                addr.with_native(|addr, addr_len| {
                    WSASendTo(
                        handle,
                        ptr,
                        len as _,
                        &mut sent,
                        0,
                        addr,
                        addr_len,
                        overlapped_ptr,
                        None,
                    )
                })
            });
            win32_result(res)
        },
    )
}

pub fn connect_named_pipe(handle: BorrowedHandle) -> IocpFuture<()> {
    IocpFuture::new(handle, (), |handle, overlapped_ptr, _| unsafe {
        let res = ConnectNamedPipe(handle as _, overlapped_ptr);
        win32_result(res)
    })
}
