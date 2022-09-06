use std::net::Ipv4Addr;
use tokio::net::{TcpListener, TcpStream};

#[test]
fn use_tokio_types_from_runtime() {
    tokio_iocp::start(async {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let task = tokio_iocp::spawn(async move {
            let _socket = TcpStream::connect(addr).await.unwrap();
        });

        // Accept a connection
        let (_socket, _) = listener.accept().await.unwrap();

        // Wait for the task to complete
        task.await.unwrap();
    });
}

#[test]
fn spawn_a_task() {
    use std::cell::RefCell;
    use std::rc::Rc;

    tokio_iocp::start(async {
        let cell = Rc::new(RefCell::new(1));
        let c = cell.clone();
        let handle = tokio_iocp::spawn(async move {
            *c.borrow_mut() = 2;
        });

        handle.await.unwrap();
        assert_eq!(2, *cell.borrow());
    });
}
