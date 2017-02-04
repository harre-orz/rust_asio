use openssl_sys::*;
use openssl::ssl;

bitflags! {
    pub flags SslOptions: u64 {
        /// Implement various bug workarounds.
        const DEFAULT_WORKGROUNDS = SSL_OP_ALL,

        /// Always create a new key when using tmp_dh parameters.
        const SINGLE_DH_USE = SSL_OP_SINGLE_DH_USE,

        /// Disable SSL v2.
        const NO_SSL_V2 = SSL_OP_NO_SSLv2,

        /// Disable SSL v3.
        const NO_SSL_V3 = SSL_OP_NO_SSLv3,

        /// Disable TLS v1.
        const NO_TLS_V1 = SSL_OP_NO_TLSv1,

        /// Disable TLS v1.1.
        const NO_TLS_V1_1 = SSL_OP_NO_TLSv1_1,

        /// Disable TLS v1.2.
        const NO_TLS_V1_2 = SSL_OP_NO_TLSv1_2,

        /// Disable compression. Compression is disabled by default.
        const NO_COMPRESSION = SSL_OP_NO_COMPRESSION,
    }
}

/// File format types.
#[repr(i32)]
pub enum FileFormat {
    // ASN.1 file.
    //ASN1 = X509_FILETYPE_ASN1,

    /// PEM file.
    PEM = X509_FILETYPE_PEM,
}

pub type SslVerifyMode = ssl::SslVerifyMode;

/// Different handshake types.
#[derive(Clone, Copy)]
pub enum Handshake {
    /// Perform handshaking as a client.
    Client,

    /// Perform handshaking as a server.
    Server,
}

/// Purpose of PEM password.
#[repr(i32)]
pub enum PasswordPurpose {
    /// The password is needed for reading/decryption.
    ForReading = 0,

    /// The password is needed for writing/encryption.
    ForWriting = 1,
}
