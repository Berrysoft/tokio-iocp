use crate::buf::*;
use windows_sys::Win32::Networking::WinSock::WSABUF;

pub struct BufWrapper<T> {
    buffer: Option<T>,
}

impl<T: IoBuf> WithBuf for BufWrapper<T> {
    type Buffer = T;

    fn new(buffer: Self::Buffer) -> Self {
        Self {
            buffer: Some(buffer),
        }
    }

    fn with_buf<R>(&self, f: impl FnOnce(*const u8, usize) -> R) -> R {
        let buffer = self.buffer.as_ref().unwrap();
        f(buffer.as_buf_ptr(), buffer.buf_len())
    }

    fn take_buf(&mut self) -> Self::Buffer {
        self.buffer.take().unwrap()
    }
}

impl<T: IoBufMut> WithBufMut for BufWrapper<T> {
    fn set_len(&mut self, len: usize) {
        self.buffer.as_mut().unwrap().set_buf_len(len)
    }

    fn with_buf_mut<R>(&mut self, f: impl FnOnce(*mut u8, usize) -> R) -> R {
        let buffer = self.buffer.as_mut().unwrap();
        f(buffer.as_buf_mut_ptr(), buffer.buf_capacity())
    }
}

impl<T: IoBuf> WithWsaBuf for BufWrapper<T> {
    fn with_wsa_buf<R>(&self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffer = self.buffer.as_ref().unwrap();
        let buffer = WSABUF {
            len: buffer.buf_len() as _,
            buf: buffer.as_buf_ptr() as _,
        };
        f(&buffer, 1)
    }
}

impl<T: IoBufMut> WithWsaBufMut for BufWrapper<T> {
    fn with_wsa_buf_mut<R>(&mut self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffer = self.buffer.as_mut().unwrap();
        let buffer = WSABUF {
            len: buffer.buf_capacity() as _,
            buf: buffer.as_buf_mut_ptr(),
        };
        f(&buffer, 1)
    }
}

pub struct VectoredBufWrapper<T> {
    buffer: Vec<T>,
}

impl<T: IoBuf> WithBuf for VectoredBufWrapper<T> {
    type Buffer = Vec<T>;

    fn new(buffer: Self::Buffer) -> Self {
        Self { buffer }
    }

    fn with_buf<R>(&self, _f: impl FnOnce(*const u8, usize) -> R) -> R {
        unimplemented!()
    }

    fn take_buf(&mut self) -> Self::Buffer {
        std::mem::take(&mut self.buffer)
    }
}

impl<T: IoBuf> WithWsaBuf for VectoredBufWrapper<T> {
    fn with_wsa_buf<R>(&self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffers = self
            .buffer
            .iter()
            .map(|buf| WSABUF {
                len: buf.buf_len() as _,
                buf: buf.as_buf_ptr() as _,
            })
            .collect::<Vec<_>>();
        f(buffers.as_ptr(), buffers.len())
    }
}

impl<T: IoBufMut> WithBufMut for VectoredBufWrapper<T> {
    fn set_len(&mut self, mut len: usize) {
        for buf in self.buffer.iter_mut() {
            let capacity = buf.buf_capacity();
            if len >= capacity {
                buf.set_buf_len(capacity);
                len -= capacity;
            } else {
                buf.set_buf_len(len);
                len = 0;
            }
        }
    }

    fn with_buf_mut<R>(&mut self, _f: impl FnOnce(*mut u8, usize) -> R) -> R {
        unimplemented!()
    }
}

impl<T: IoBufMut> WithWsaBufMut for VectoredBufWrapper<T> {
    fn with_wsa_buf_mut<R>(&mut self, f: impl FnOnce(*const WSABUF, usize) -> R) -> R {
        let buffers = self
            .buffer
            .iter_mut()
            .map(|buf| WSABUF {
                len: buf.buf_capacity() as _,
                buf: buf.as_buf_mut_ptr() as _,
            })
            .collect::<Vec<_>>();
        f(buffers.as_ptr(), buffers.len())
    }
}
