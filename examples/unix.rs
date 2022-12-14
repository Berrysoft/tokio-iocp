use scopeguard::defer;
use tokio_iocp::net::{UnixListener, UnixStream};

fn main() {
    tokio_iocp::start(async {
        let path = format!("{}/unix-example.sock", std::env::var("TEMP").unwrap());
        let listener = UnixListener::bind(&path).unwrap();
        defer! {
            std::fs::remove_file(&path).unwrap();
        }

        let addr = listener.local_addr().unwrap();

        let tx = UnixStream::connect_addr(addr).unwrap();
        let (rx, client_addr) = listener.accept().await.unwrap();
        println!(
            "Server local addr: {}\nServer peer addr: {}",
            addr, client_addr
        );

        println!(
            "Client local addr: {}\nClient peer addr: {}",
            tx.local_addr().unwrap(),
            tx.peer_addr().unwrap()
        );

        tx.send("Hello world!").await.0.unwrap();

        let buffer = Vec::with_capacity(64);
        let (n, buffer) = rx.recv(buffer).await;
        n.unwrap();
        println!("{}", String::from_utf8(buffer).unwrap());
    });
}
