#![feature(read_buf)]

use std::{io::BorrowedBuf, mem::MaybeUninit, net::Ipv4Addr};
use tokio_iocp::net::{TcpListener, TcpStream};

fn main() {
    tokio_iocp::start(async {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, (rx, _)) = tokio::try_join!(TcpStream::connect(addr), listener.accept()).unwrap();
        tx.send("Hello world!").await.0.unwrap();

        static mut BUFFER: &mut [MaybeUninit<u8>] = &mut [MaybeUninit::uninit(); 64];
        let buffer = unsafe { BorrowedBuf::from(&mut BUFFER[..]) };
        let (n, buffer) = rx.recv(buffer).await;
        assert_eq!(n.unwrap(), buffer.len());
        println!("{}", String::from_utf8_lossy(buffer.filled()));
    });
}
