use std::time::Duration;

use tokio_iocp::net::named_pipe::{ClientOptions, PipeMode, ServerOptions};

const PIPE_NAME: &str = r"\\.\pipe\tokio-iocp-named-pipe";

fn main() {
    let server = std::thread::spawn(|| {
        tokio_iocp::start(async {
            let server = ServerOptions::new()
                .pipe_mode(PipeMode::Message)
                .max_instances(5)
                .create(PIPE_NAME)
                .unwrap();
            println!("{:?}", server.info().unwrap());

            server.connect().await.unwrap();
            server.write("Hello world!").await.0.unwrap();
        })
    });
    let client = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(1));
        tokio_iocp::start(async {
            let client = ClientOptions::new().open(PIPE_NAME).unwrap();
            println!("{:?}", client.info().unwrap());

            let buffer = Vec::with_capacity(64);
            let (n, buffer) = client.read(buffer).await;
            n.unwrap();
            println!("{}", String::from_utf8(buffer).unwrap());
        })
    });
    server.join().unwrap();
    client.join().unwrap();
}
