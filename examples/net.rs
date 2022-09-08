use std::net::Ipv4Addr;
use tokio_iocp::net::{TcpListener, TcpStream};

fn main() {
    tokio_iocp::start(async {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();

        let (tx, (rx, _)) = tokio::try_join!(TcpStream::connect(addr), listener.accept()).unwrap();
        tx.send("Hello world!").await.0.unwrap();

        let buffer = Vec::with_capacity(64);
        let (n, buffer) = rx.recv(buffer).await;
        n.unwrap();
        println!("{}", String::from_utf8(buffer).unwrap());
    });
}
