# Next Gen Transport Mockup

CoAP-based mockup for Next Gen Transport task group.

Please excuse the relatively poor quality of the code. This was mostly written hackathon-style in the context of the ESTA/NATEAC summer meetings in July 2024.

## Slides

The slides that were presented at the Next-Gen Transport task group meeting on 12 July 2024 are in the `slides` directory.

## Building

Building and running requires a Rust toolchain. You can install it from [here](https://www.rust-lang.org/tools/install).

Inside the directory for each root-level crate, build the project with `cargo build`. Then, you can run it with `cargo run`.

## Crates

This project is divided into 4 crates:

- `create_certs`: Creates a self-signed root of trust and certificates for the other components (`arbiter`, `controller`, `device`), as well as another self-signed certificate which can be used to demonstrate invalid certificate handling.
- `arbiter`: Runs the arbiter service as a CoAP server with DTLS.
- `device`: Runs the device service as a combination CoAP client/server with DTLS.
- `controller`: Runs the controller as a CoAP client with DTLS. Has a simple text interface to perform commands.

All devices require `config.json` files, the contents of which can be determined by inspecting the `config.rs` source file.

## Certificates Cheat Sheet

### Displaying the contents of a certificate

```
$ openssl x509 -in arbiter-cert.pem -text -noout
```
