# pipewire-flow

Safe Rust bindings to [pipewire-flow], a C library that gives PipeWire
applications a simpler API for audio and video capture, audio playback, and
multi-port filters that bundle every input into one graph cycle. It hides the thread loop, SPA POD format negotiation, and buffer
dequeue/queue plumbing behind owned handles that return `Result`.

PipeWire is Linux-only, so this crate builds and runs there.

```toml
[dependencies]
pipewire-flow = "0.2"
```

Capture from the default microphone for five seconds:

```rust,no_run
use pipewire_flow::{AudioConfig, Stream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = Stream::audio_capture(|buf| {
        if let Some(data) = buf.data() {
            println!("{} bytes (pts={:?} ns)", data.len(), buf.pts());
        }
    })?;

    stream.set_audio_config(&AudioConfig::new(48_000, 2))?;
    stream.start()?;
    std::thread::sleep(std::time::Duration::from_secs(5));
    stream.stop(false)?;
    Ok(())
}
```

Every callback runs on a thread the handle owns, so each closure is
`Send + 'static`, and the buffers it receives borrow the graph's memory for
one call only. Buffer callbacks run on the real-time data thread and must not
block. Dropping a `Stream` or `Filter` stops it and joins its threads.

## Building

The C library comes from one of two places. By default `pipewire-flow-sys`
probes pkg-config for an installed `pipewire-flow` >= 0.12.0; the `vendored`
feature builds the C sources the `-sys` crate ships, which needs Meson, Ninja
and `libpipewire-0.3` >= 0.3.50 development files.

```sh
sudo apt-get install meson ninja-build pkg-config libpipewire-0.3-dev libspa-0.2-dev
cargo build --features vendored
```

## License

MIT, matching the C library.

[pipewire-flow]: https://github.com/pipewire-flow/pipewire-flow
