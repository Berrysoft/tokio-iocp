use criterion::{criterion_group, criterion_main, Criterion};

criterion_group!(read, std, tokio, iocp);
criterion_main!(read);

fn std(c: &mut Criterion) {
    c.bench_function("std", |b| {
        b.iter(|| {
            use std::io::Read;

            let mut file = std::fs::File::open("Cargo.toml").unwrap();
            let mut buffer = Vec::with_capacity(1024);
            file.read_to_end(&mut buffer).unwrap();
            buffer
        })
    });
}

fn tokio(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    c.bench_function("tokio", |b| {
        b.to_async(&runtime).iter(|| async {
            use tokio::io::AsyncReadExt;

            let mut file = tokio::fs::File::open("Cargo.toml").await.unwrap();
            let mut buffer = Vec::with_capacity(1024);
            file.read_to_end(&mut buffer).await.unwrap();
            buffer
        })
    });
}

fn iocp(c: &mut Criterion) {
    let runtime = tokio_iocp::runtime::Runtime::new().unwrap();

    c.bench_function("iocp", |b| {
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
}
