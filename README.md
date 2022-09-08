# tokio-iocp

[![crates.io](https://img.shields.io/crates/v/tokio-iocp)](https://crates.io/crates/tokio-iocp)
[![docs.rs](https://img.shields.io/badge/docs.rs-tokio--iocp-latest)](https://docs.rs/tokio-iocp)

This crate, inspired by [`tokio-uring`], provides [IOCP] for [Tokio] by exposing a new Runtime that is
compatible with Tokio but also can drive [IOCP]-backed resources. Any
library that works with [Tokio] also works with `tokio-iocp`. The crate
provides new resource types that work with [IOCP].

[IOCP]: https://docs.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports
[Tokio]: https://github.com/tokio-rs/tokio
[`tokio-uring`]: https://github.com/tokio-rs/tokio-uring

# Getting started

Using `tokio-iocp` requires starting a [`tokio-iocp`] runtime. This
runtime internally manages the main (single-threaded) Tokio runtime and a IOCP driver.

```rust
use tokio_iocp::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio_iocp::start(async {
        // Open a file
        let file = File::open("hello.txt")?;

        let buf = Vec::with_capacity(4096);
        // Read some data, the buffer is passed by ownership and
        // submitted to the kernel. When the operation completes,
        // we get the buffer back.
        let (res, buf) = file.read_at(buf, 0).await;
        let n = res?;

        // Display the contents
        println!("{:?}", &buf);

        Ok(())
    })
}
```
## Requirements
Windows.
 
## Project status

The `tokio-iocp` project is still very young. Currently, we are focusing on
supporting filesystem and network operations. We are looking forward to your contributions!

## License

This project is licensed under the [MIT license].

[MIT license]: LICENSE
