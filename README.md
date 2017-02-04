# asyncio - ASynchronous Input/Output library

[![Build Status](https://travis-ci.org/harre-orz/rust_asio.svg?branch=master)](https://travis-ci.org/harre-orz/rust_asio)

The `asyncio` is not compatible to `Rust` stable version (ver 1.13). Please install a `Rust-nightly`.

This crate is on [github](https://github.com/harre-orz/rust_asio.git "github") and can be used by adding `asyncio` to the dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
rust_asio = "*"
```

[Documentation](http://harre-orz.github.io/rust_asio/asyncio/ "Documentation")

## Features
 - Proactor design pattern based thread-safe asynchronous I/O.
 - Does not dependent on the number of threads.
 - Supported protocol is in TCP, UDP, Unix-domain socket and etc.
 - Supported timer is in system timer, steady timer.
 - Supported File descriptor socket.
 - Supported Generic protocol socket.
 - Supported Signal Handing. (Linux only)
 - Supported Serial-port

## Platforms

Currently supported platforms:
 - Linux (kernel version >=2.6.27)
 - MacOS X

## TODO list
 1. BSD will support.
 2. SSL will support.
 3. Windows will support.
