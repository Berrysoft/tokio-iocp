use tempfile::NamedTempFile;
use tokio_iocp::{buf::*, fs::File};

// Ignore this test because we need to keep the buffer until
// the operation succeeds.
#[test]
#[ignore]
fn complete_ops_on_drop() {
    use std::sync::Arc;

    struct MyBuf {
        data: Vec<u8>,
        _ref_cnt: Arc<()>,
    }

    unsafe impl IoBuf for MyBuf {
        fn as_buf_ptr(&self) -> *const u8 {
            self.data.as_buf_ptr()
        }

        fn buf_len(&self) -> usize {
            self.data.buf_len()
        }

        fn buf_capacity(&self) -> usize {
            self.data.buf_capacity()
        }
    }

    unsafe impl IoBufMut for MyBuf {
        fn as_buf_mut_ptr(&mut self) -> *mut u8 {
            self.data.as_buf_mut_ptr()
        }

        fn set_buf_init(&mut self, pos: usize) {
            self.data.set_buf_init(pos);
        }
    }

    // Used to test if the buffer dropped.
    let ref_cnt = Arc::new(());

    let tempfile = tempfile();

    let vec = vec![0; 50 * 1024 * 1024];
    let mut file = std::fs::File::create(tempfile.path()).unwrap();
    std::io::Write::write_all(&mut file, &vec).unwrap();

    let file = tokio_iocp::start(async {
        let file = File::open(tempfile.path()).unwrap();
        poll_once(async {
            file.read_at(
                MyBuf {
                    data: Vec::with_capacity(64 * 1024),
                    _ref_cnt: ref_cnt.clone(),
                },
                25 * 1024 * 1024,
            )
            .await
            .0
            .unwrap();
        })
        .await;

        file
    });

    assert_eq!(Arc::strong_count(&ref_cnt), 1);

    drop(file);
}

#[test]
fn too_many_submissions() {
    let tempfile = tempfile();

    tokio_iocp::start(async {
        let file = File::create(tempfile.path()).unwrap();
        for _ in 0..600 {
            poll_once(async {
                file.write_at("hello world", 0).await.0.unwrap();
            })
            .await;
        }
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
