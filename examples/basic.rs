use tempfile::NamedTempFile;
use tokio_iocp::fs::File;

fn main() -> std::io::Result<()> {
    tokio_iocp::start(async {
        let file = File::open("Cargo.toml")?;
        let buf = vec![0; 400];
        let (n, mut buf) = file.read_at(buf, 0).await;
        let n = n?;
        buf.resize(n, 0);
        print!("{}", String::from_utf8_lossy(&buf));
        let file = File::create(NamedTempFile::new()?)?;
        let (n, _) = file.write_at(buf, 0).await;
        let n = n?;
        println!("Wrote {} bytes.", n);
        Ok(())
    })
}
