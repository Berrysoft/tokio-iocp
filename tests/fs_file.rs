use std::io::prelude::*;
use tempfile::NamedTempFile;
use tokio_iocp::fs::File;

const HELLO: &[u8] = b"hello world...";

async fn read_hello(file: &File) {
    let buf = vec![0; 1024];
    let (res, buf) = file.read_at(buf, 0).await;
    let n = res.unwrap();

    assert_eq!(n, HELLO.len());
    assert_eq!(&buf[..n], HELLO);
}

#[test]
fn basic_read() {
    tokio_iocp::start(async {
        let mut tempfile = tempfile();
        tempfile.write_all(HELLO).unwrap();

        let file = File::open(tempfile.path()).unwrap();
        read_hello(&file).await;
    });
}

#[test]
fn basic_write() {
    tokio_iocp::start(async {
        let tempfile = tempfile();

        let file = File::create(tempfile.path()).unwrap();

        file.write_at(HELLO, 0).await.0.unwrap();

        let file = std::fs::read(tempfile.path()).unwrap();
        assert_eq!(file, HELLO);
    });
}

#[test]
fn cancel_read() {
    tokio_iocp::start(async {
        let mut tempfile = tempfile();
        tempfile.write_all(HELLO).unwrap();

        let file = File::open(tempfile.path()).unwrap();

        // Poll the future once, then cancel it
        poll_once(async { read_hello(&file).await }).await;

        read_hello(&file).await;
    });
}

#[test]
fn drop_open() {
    tokio_iocp::start(async {
        let tempfile = tempfile();
        let _ = File::create(tempfile.path());

        // Do something else
        let file = File::create(tempfile.path()).unwrap();

        file.write_at(HELLO, 0).await.0.unwrap();

        let file = std::fs::read(tempfile.path()).unwrap();
        assert_eq!(file, HELLO);
    });
}

fn tempfile() -> NamedTempFile {
    NamedTempFile::new().unwrap()
}

async fn poll_once(future: impl std::future::Future) {
    use std::{future::poll_fn, task::Poll};
    use tokio::pin;

    pin!(future);

    poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
}
