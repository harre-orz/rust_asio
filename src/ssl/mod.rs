mod ffi;

use openssl_sys::X509_STORE_CTX;
pub struct SslVerifyContext(*mut X509_STORE_CTX);

#[derive(Clone, Copy)]
/// Different handshake types.
pub enum Handshake {
    /// Perform handshaking as a client.
    Client,

    /// Perform handshaking as a server.
    Server,
}

mod error;
pub use self::error::*;

mod context;
pub use self::context::*;

mod engine;
pub use self::engine::*;

mod stream;
pub use self::stream::*;
