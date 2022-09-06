use std::net::IpAddr;
use tokio_iocp::net::{TcpListener, TcpStream};

fn main() {
    tokio_iocp::start(async {
        let listener = TcpListener::bind(("127.0.0.1".parse::<IpAddr>().unwrap(), 10086)).unwrap();
        let addr = listener.local_addr().unwrap();

        let task = tokio_iocp::spawn(async move {
            let _socket = TcpStream::connect(addr).unwrap();
        });

        // Accept a connection
        let (_socket, _) = listener.accept().await.unwrap();

        // Wait for the task to complete
        task.await.unwrap();
    });
}
