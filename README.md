[![Continuous integration](https://github.com/alekseysidorov/static-box/actions/workflows/ci.yml/badge.svg)](https://github.com/alekseysidorov/static-box/actions/workflows/ci.yml)
![Crates.io](https://img.shields.io/crates/v/static-box)
[![API reference](https://docs.rs/static-box/badge.svg)](https://docs.rs/static-box/)

# Overview

This crate allows saving DST objects in the provided buffer. It allows users to create global dynamic objects on a `no_std` environment without a global allocator.

```rust
use static_box::Box;

struct Uart1Rx {
    // Implementation details...
}

impl SerialWrite for Uart1Rx {
    fn write(&mut self, _byte: u8) {
        // Implementation details
    }
}
let rx = Uart1Rx { /* ... */ };

let mut writer = Box::<dyn SerialWrite, [u8; 32]>::new(rx);
writer.write_str("Hello world!");
```

This implementation is inspired by the [`thin_box`](https://github.com/rust-lang/rust/blob/5ade3fe32c8a742504aaddcbe0d6e498f8eae11d/library/core/tests/ptr.rs#L561) example in the `rustc` tests repository.

# Minimum Supported `rustc` Version

This crate uses following unstable features:
- [`ptr_metadata`](https://doc.rust-lang.org/unstable-book/library-features/ptr-metadata.html)
- [`unsize`](https://doc.rust-lang.org/unstable-book/library-features/unsize.html)

In other words, the crate's supported **nightly** `rustc` version is `1.53.0`, but there is no guarantee that this code will work fine on the newest versions.

# License

Dual-licensed to be compatible with the Rust project.

Licensed under the Apache License, Version 2.0 http://www.apache.org/licenses/LICENSE-2.0 or the MIT license http://opensource.org/licenses/MIT, at your option. This file may not be copied, modified, or distributed except according to those terms.
