# Simp UART Protocol Library

A simple and reliable UART communication protocol implemented in Rust. This library provides functionality to send and receive data packets over UART, with optional acknowledgment (ACK) handling, making it suitable for embedded systems like the ESP32 and standard PC applications.

## Features

- Packet-based communication with start and end delimiters.
- Automatic escaping and unescaping of special bytes.
- Checksum for error detection.
- Support for sending packets with or without waiting for ACK.
- Compatible with both embedded systems (e.g., ESP32) and standard PCs (Windows/Linux).

## Getting Started

### Prerequisites

- **Rust**: Make sure you have Rust installed. You can install it from [rust-lang.org](https://www.rust-lang.org/).
- **ESP32**: If you plan to use this library on an ESP32, read [this guide](https://docs.esp-rs.org/book/) first.

### Installation

Add this library to your `Cargo.toml`:

```toml
[dependencies]
simp_protocol = { version = "0.1.1" }  # Adjust the path as necessary
```

### How to use?

Please check out `examples` directory as it contains all you need to start using this library.

### License

This library is published with MIT license.