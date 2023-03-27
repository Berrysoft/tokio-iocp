use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::NamedTempFile;

criterion_group!(fs, read, write);
criterion_main!(fs);

fn read(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");

    group.bench_function("std", |b| {
        b.iter(|| {
            use std::io::Read;

            let mut file = std::fs::File::open("Cargo.toml").unwrap();
            let mut buffer = Vec::with_capacity(1024);
            file.read_to_end(&mut buffer).unwrap();
            buffer
        })
    });

    group.bench_function("tokio", |b| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        b.to_async(&runtime).iter(|| async {
            use tokio::io::AsyncReadExt;

            let mut file = tokio::fs::File::open("Cargo.toml").await.unwrap();
            let mut buffer = Vec::with_capacity(1024);
            file.read_to_end(&mut buffer).await.unwrap();
            buffer
        })
    });

    group.bench_function("iocp", |b| {
        let runtime = tokio_iocp::runtime::Runtime::new().unwrap();
        b.to_async(&runtime).iter(|| async {
            let file = tokio_iocp::fs::File::open("Cargo.toml").unwrap();
            let mut buffer = Vec::with_capacity(1024);
            loop {
                let old_len = buffer.len();
                let (n, sbuf) = file.read_at(buffer, old_len).await;
                buffer = sbuf;
                let n = n.unwrap();
                if n == 0 {
                    break;
                }
            }
            buffer
        })
    });

    group.finish();
}

static CONTENT: &[u8] = include_bytes!("../Cargo.toml");

fn write(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");

    group.bench_function("std", |b| {
        let temp_file = NamedTempFile::new().unwrap();
        b.iter(|| {
            use std::io::Write;

            let mut file = std::fs::File::create(temp_file.path()).unwrap();
            file.write_all(CONTENT).unwrap();
        })
    });

    group.bench_function("tokio", |b| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let temp_file = NamedTempFile::new().unwrap();
        b.to_async(&runtime).iter(|| async {
            use tokio::io::AsyncWriteExt;

            let mut file = tokio::fs::File::create(temp_file.path()).await.unwrap();
            file.write_all(CONTENT).await.unwrap();
        })
    });

    group.bench_function("iocp", |b| {
        let runtime = tokio_iocp::runtime::Runtime::new().unwrap();
        let temp_file = NamedTempFile::new().unwrap();
        b.to_async(&runtime).iter(|| async {
            let file = tokio_iocp::fs::File::create(temp_file.path()).unwrap();
            let mut pos = 0;
            while pos < CONTENT.len() {
                let (res, _) = file.write_at(&CONTENT[pos..], pos).await;
                pos += res.unwrap();
            }
        })
    });

    group.finish()
}
