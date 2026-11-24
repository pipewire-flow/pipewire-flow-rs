//! Wires a capture stream to a named device with no session manager
//! involved, pausing so the graph can be inspected with `pw-link -l` before
//! and after.
//!
//! Usage: `stream_manual_link <device> [other-device]`, where the second
//! device re-targets the stream. Names come from `wpctl status`; an
//! object.serial works too.

use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pipewire_flow::{AudioConfig, Routing, Stream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(device) = args.next() else {
        eprintln!("usage: stream_manual_link <device> [other-device]");
        std::process::exit(1);
    };
    let other = args.next();

    let buffers = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&buffers);
    let stream = Stream::audio_capture(move |_| {
        count.fetch_add(1, Ordering::Relaxed);
    })?;
    // Nothing re-links the stream on its own; that decision is ours to make.
    stream.set_error_callback(|err| eprintln!("device lost ({err}); nothing will re-route it"))?;

    // From here the session manager will not wire this stream.
    stream.set_routing(Routing::Manual)?;
    stream.set_audio_config(&AudioConfig::new(48_000, 2))?;
    stream.start()?;

    println!("started, wired to nothing. check: pw-link -l | grep {device}");
    wait_for_enter("press Enter to link... ")?;
    stream.link(&device)?;
    println!("linked to {device}; check pw-link -l again");

    wait_for_enter("press Enter to stop and start (the links must survive)... ")?;
    stream.stop(false)?;
    println!("stopped; the links are still there");
    stream.start()?;
    println!("started again on the same device");

    if let Some(other) = other {
        wait_for_enter("press Enter to re-target... ")?;
        stream.unlink()?;
        match stream.link(&other) {
            Ok(()) => println!("moved to {other}"),
            Err(err) => eprintln!("could not move to {other}: {err}"),
        }
    }

    println!("capturing for ten seconds...");
    std::thread::sleep(Duration::from_secs(10));
    println!("{} buffers received", buffers.load(Ordering::Relaxed));
    stream.stop(false)?;
    Ok(())
}

fn wait_for_enter(prompt: &str) -> std::io::Result<()> {
    print!("{prompt}");
    std::io::stdout().flush()?;
    std::io::stdin().lock().read_line(&mut String::new())?;
    Ok(())
}
