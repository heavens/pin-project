# pin-project

[![crates.io](https://img.shields.io/crates/v/pin-project.svg?style=flat-square&logo=rust)](https://crates.io/crates/pin-project)
[![docs.rs](https://img.shields.io/badge/docs.rs-pin--project-blue?style=flat-square)](https://docs.rs/pin-project)
[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue.svg?style=flat-square)](#license)
[![rustc](https://img.shields.io/badge/rustc-1.37+-blue.svg?style=flat-square)](https://www.rust-lang.org)
[![build status](https://img.shields.io/github/workflow/status/taiki-e/pin-project/CI/master?style=flat-square)](https://github.com/taiki-e/pin-project/actions?query=workflow%3ACI+branch%3Amaster)

A crate for safe and ergonomic [pin-projection].

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
pin-project = "1"
```

*Compiler support: requires rustc 1.37+*

## Examples

[`#[pin_project]`][`pin_project`] attribute creates projection types
covering all the fields of struct or enum.

```rust
use pin_project::pin_project;
use std::pin::Pin;

#[pin_project]
struct Struct<T, U> {
    #[pin]
    pinned: T,
    unpinned: U,
}

impl<T, U> Struct<T, U> {
    fn method(self: Pin<&mut Self>) {
        let this = self.project();
        let _: Pin<&mut T> = this.pinned; // Pinned reference to the field
        let _: &mut U = this.unpinned; // Normal reference to the field
    }
}
```

[*code like this will be generated*][struct-default-expanded]

To use `#[pin_project]` on enums, you need to name the projection type
returned from the method.

```rust
use pin_project::pin_project;
use std::pin::Pin;

#[pin_project(project = EnumProj)]
enum Enum<T, U> {
    Pinned(#[pin] T),
    Unpinned(U),
}

impl<T, U> Enum<T, U> {
    fn method(self: Pin<&mut Self>) {
        match self.project() {
            EnumProj::Pinned(x) => {
                let _: Pin<&mut T> = x;
            }
            EnumProj::Unpinned(y) => {
                let _: &mut U = y;
            }
        }
    }
}
```

[*code like this will be generated*][enum-default-expanded]

See [documentation](https://docs.rs/pin-project) for more details, and
see [examples] directory for more examples and generated code.

[`pin_project`]: https://docs.rs/pin-project/1/pin_project/attr.pin_project.html
[enum-default-expanded]: examples/enum-default-expanded.rs
[examples]: examples/README.md
[pin-projection]: https://doc.rust-lang.org/std/pin/index.html#projections-and-structural-pinning
[struct-default-expanded]: examples/struct-default-expanded.rs

## Related Projects

* [pin-project-lite]: A lightweight version of pin-project written with declarative macros.

[pin-project-lite]: https://github.com/taiki-e/pin-project-lite

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
