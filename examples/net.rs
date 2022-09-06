use std::net::Ipv4Addr;
use tokio_iocp::net::{TcpListener, TcpStream};

fn main() {
    tokio_iocp::start(async {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();

        let task = tokio_iocp::spawn(async move {
            let socket = TcpStream::connect(addr).await.unwrap();
            socket.send("Hello world!").await.0.unwrap();
        });

        // Accept a connection
        let (socket, _) = listener.accept().await.unwrap();
        let buffer = Vec::with_capacity(64);
        let (n, buffer) = socket.recv(buffer).await;
        n.unwrap();
        println!("{}", String::from_utf8(buffer).unwrap());

        // Wait for the task to complete
        task.await.unwrap();
    });
}
