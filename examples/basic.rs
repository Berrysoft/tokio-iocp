use tempfile::NamedTempFile;
use tokio_iocp::{fs::File, IoResult};

fn main() -> IoResult<()> {
    tokio_iocp::start(async {
        let file = File::open("Cargo.toml")?;
        let buf = Vec::with_capacity(400);
        let (n, buf) = file.read_at(buf, 0).await;
        let n = n?;
        assert_eq!(n, buf.len());
        print!("{}", String::from_utf8_lossy(&buf));
        let file = File::create(NamedTempFile::new()?)?;
        let (n, _) = file.write_at(buf, 0).await;
        let n = n?;
        println!("Wrote {} bytes.", n);
        Ok(())
    })
}
