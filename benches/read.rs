#![feature(test)]

use test::Bencher;

extern crate test;

#[bench]
fn std(b: &mut Bencher) {
    let mut file = std::fs::File::open("Cargo.toml").unwrap();
    b.iter(|| {
        use std::io::Read;
        let mut buffer = Vec::with_capacity(1024);
        let mut sub_buffer = vec![0u8; 512];
        loop {
            let n = file.read(&mut sub_buffer).unwrap();
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&sub_buffer[..n]);
        }
        buffer
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
            let mut sub_buffer = vec![0u8; 512];
            loop {
                let n = file.read(&mut sub_buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                buffer.extend_from_slice(&sub_buffer[..n]);
            }
            buffer
        })
    })
}

#[bench]
fn iocp(b: &mut Bencher) {
    let file = tokio_iocp::fs::File::open("Cargo.toml").unwrap();
    let runtime = tokio_iocp::runtime::Runtime::new().unwrap();
    b.iter(|| {
        runtime.block_on(async {
            let mut buffer = Vec::with_capacity(1024);
            let mut sub_buffer = Vec::with_capacity(512);
            let mut n;
            let mut len = 0;
            loop {
                (n, sub_buffer) = file.read_at(sub_buffer, len).await;
                let new_n = n.unwrap();
                if new_n == 0 {
                    break;
                }
                buffer.extend_from_slice(&sub_buffer[..new_n]);
                len += new_n;
            }
            buffer
        })
    })
}
