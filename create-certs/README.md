# Certificate Creation Utility

The Rust TLS ecosystem is not quite compatible with OpenSSL; in general, they move aggressively to newer standards and do not support older ones, whereas often it is [needlessly difficult](https://serverfault.com/questions/845766/generating-a-self-signed-cert-with-openssl-that-works-in-chrome-58) or [impossible](https://github.com/openssl/openssl/issues/10468) to use OpenSSL in a way that supports these newer formats.

This utility generates a set of certificates for the components of this project; a self-signed root certificate, and two certificate/key pairs that are signed by the root certificate (one for the arbiter and one for the client).

To use it, simply `cargo run` and the certificates will be written to an `out` subfolder.
