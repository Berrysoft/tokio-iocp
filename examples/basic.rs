use tokio_iocp::fs::File;

fn main() {
    tokio_iocp::start(async {
        let file = File::open("Cargo.toml").unwrap();
        let buf = vec![0; 300];
        let (n, buf) = file.read_at(buf, 0).await;
        let n = n.unwrap();
        print!("{}", String::from_utf8_lossy(&buf[..n]));
    })
}
