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

    fn read(file: &mut std::fs::File) -> Vec<u8> {
        use std::io::Read;
        let mut buffer = Vec::with_capacity(1024);
        file.read_to_end(&mut buffer).unwrap();
        buffer
    }

    b.iter(|| {
        runtime.block_on(async {
            for _i in 0..100 {
                test::black_box(read(&mut file));
            }
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

    async fn read(file: &mut tokio::fs::File) -> Vec<u8> {
        use tokio::io::AsyncReadExt;
        let mut buffer = Vec::with_capacity(1024);
        file.read_to_end(&mut buffer).await.unwrap();
        buffer
    }

    b.iter(|| {
        runtime.block_on(async {
            for _i in 0..100 {
                test::black_box(read(&mut file).await);
            }
        })
    })
}

#[bench]
fn iocp(b: &mut Bencher) {
    let file = tokio_iocp::fs::File::open("Cargo.toml").unwrap();
    let runtime = tokio_iocp::runtime::Runtime::new().unwrap();

    async fn read(file: &tokio_iocp::fs::File) -> Vec<u8> {
        let buffer = Vec::with_capacity(1024);
        let (n, buffer) = file.read_at(buffer, 0).await;
        n.unwrap();
        buffer
    }

    b.iter(|| {
        runtime.block_on(async {
            use futures_util::future::join_all;
            let buffers = join_all((0..100).map(|_| read(&file))).await;
            test::black_box(buffers)
        })
    })
}
