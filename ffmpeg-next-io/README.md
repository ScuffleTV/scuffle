# ffmpeg-next-io

This is a helper library for [ffmpeg-next](https://crates.io/crates/ffmpeg-next) that provides a general purpose IO implementation for bindings.

This library provides a wrapper for any struct that implements [`std::io::Read`](https://doc.rust-lang.org/std/io/trait.Read.html) and [`std::io::Write`](https://doc.rust-lang.org/std/io/trait.Write.html), with seeking support for [`std::io::Seek`](https://doc.rust-lang.org/std/io/trait.Seek.html).

We also have a channel compatability layer for many popular channels, including [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/index.html), [`crossbeam-channel`](https://crates.io/crates/crossbeam-channel) and [`tokio`](https://crates.io/crates/tokio).

This library allows you to write to FFmpeg from a different thread. Meaning you can use FFmpeg in a async runtime and pass data from the async context to the sync context without blocking or having to write to a file first.

Without this libary you would be required to write to a tempory file or network socket or some unix pipe, which is not ideal. This library provides a much needed in-memory solution for ffmpeg-next.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
ffmpeg-next-io = "0.1.0"
```

## Example

```rust
use ffmpeg_next_io::Input;

let bytes = include_bytes!("../input.mp4"); // or any other source of bytes

let mut input = Input::seekable(std::io::Cursor::new(bytes));

// now you can use `input` as an input for ffmpeg-next
```

For more examples, see the [examples](./examples) directory.

## License

This project is licensed under the MIT license ([LICENSE](./LICENSE.md) or http://opensource.org/licenses/MIT).
