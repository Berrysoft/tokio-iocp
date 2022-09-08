use scopeguard::defer;
use tokio_iocp::net::{UnixListener, UnixSocketAddr, UnixStream};

fn main() {
    tokio_iocp::start(async {
        let path = format!("{}/unix-example.sock", std::env::var("TEMP").unwrap());
        let listener = UnixListener::bind(UnixSocketAddr::from_pathname(&path).unwrap()).unwrap();
        defer! {
            std::fs::remove_file(&path).unwrap();
        }

        let addr = listener.local_addr().unwrap();
        println!("Local addr: {}", addr);

        let tx = UnixStream::connect(
            UnixSocketAddr::from_abstract_namespace(b"unix-client.sock").unwrap(),
            addr,
        )
        .unwrap();
        let rx = listener.accept().await.unwrap();

        println!(
            "Local addr: {}\nPeer addr: {}",
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
