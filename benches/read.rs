#![feature(test)]

use test::Bencher;

extern crate test;

#[bench]
fn std(b: &mut Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut file = std::fs::File::open("Cargo.toml").unwrap();

    b.iter(|| {
        runtime.block_on(async {
            use std::io::Read;
            let mut buffer = Vec::with_capacity(1024);
            file.read_to_end(&mut buffer).unwrap();
            buffer
        })
    });
}

#[bench]
fn tokio(b: &mut Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut file = runtime.block_on(async { tokio::fs::File::open("Cargo.toml").await.unwrap() });

    b.iter(|| {
        runtime.block_on(async {
            use tokio::io::AsyncReadExt;
            let mut buffer = Vec::with_capacity(1024);
            file.read_to_end(&mut buffer).await.unwrap();
            buffer
        })
    })
}

#[bench]
fn iocp(b: &mut Bencher) {
    let size = std::fs::metadata("Cargo.toml").unwrap().len() as usize;
    let file = tokio_iocp::fs::File::open("Cargo.toml").unwrap();
    let runtime = tokio_iocp::runtime::Runtime::new().unwrap();

    b.iter(|| {
        runtime.block_on(async {
            use tokio_iocp::buf::IoBuf;
            let mut buffer = Vec::with_capacity(1024);
            while buffer.len() < size {
                let old_len = buffer.len();
                let (n, sbuf) = file.read_at(buffer.slice(old_len..), old_len).await;
                buffer = sbuf.into_inner();
                let n = n.unwrap();
                if n == 0 {
                    break;
                }
            }
            buffer
        })
    })
}
