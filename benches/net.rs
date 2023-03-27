use criterion::{criterion_group, criterion_main, Criterion};

criterion_group!(net, tcp);
criterion_main!(net);

fn tcp(c: &mut Criterion) {
    const PACKET_LEN: usize = 65536;
    static PACKET: &[u8] = &[1u8; PACKET_LEN];

    let mut group = c.benchmark_group("tcp");

    group.bench_function("tokio", |b| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        b.to_async(&runtime).iter(|| async {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let tx = tokio::net::TcpStream::connect(addr);
            let rx = listener.accept();
            let (mut tx, (mut rx, _)) = tokio::try_join!(tx, rx).unwrap();
            tx.write_all(PACKET).await.unwrap();
            let mut buffer = Vec::with_capacity(PACKET_LEN);
            while buffer.len() < PACKET_LEN {
                rx.read_buf(&mut buffer).await.unwrap();
            }
            buffer
        })
    });

    group.bench_function("iocp", |b| {
        let runtime = tokio_iocp::runtime::Runtime::new().unwrap();
        b.to_async(&runtime).iter(|| async {
            let listener = tokio_iocp::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let tx = tokio_iocp::net::TcpStream::connect(addr);
            let rx = listener.accept();
            let (tx, (rx, _)) = tokio::try_join!(tx, rx).unwrap();
            {
                let mut pos = 0;
                while pos < PACKET_LEN {
                    let (res, _) = tx.send(&PACKET[pos..]).await;
                    pos += res.unwrap();
                }
            }
            {
                let mut buffer = Vec::with_capacity(PACKET_LEN);
                let mut res;
                while buffer.len() < PACKET_LEN {
                    (res, buffer) = rx.recv(buffer).await;
                    res.unwrap();
                }
                buffer
            }
        })
    });

    group.finish();
}
