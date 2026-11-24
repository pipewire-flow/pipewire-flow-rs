//! Captures MJPEG from the default camera and prints each frame's size,
//! which varies because the frames are compressed.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pipewire_flow::{PixelFormat, Stream, VideoConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = Stream::video_capture(|buf| {
        println!(
            "MJPEG frame: {} bytes (pts={:?} ns)",
            buf.data().map_or(0, <[u8]>::len),
            buf.pts()
        );
    })?;

    let lost = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&lost);
    stream.set_error_callback(move |err| {
        eprintln!("source lost: {err}");
        flag.store(true, Ordering::Relaxed);
    })?;

    // A camera that lacks MJPEG at this size refuses the format here.
    stream.set_video_config(&VideoConfig::new(1280, 720, PixelFormat::Mjpg).with_fps(30))?;
    stream.start()?;

    let deadline = Instant::now() + Duration::from_secs(10);
    while !lost.load(Ordering::Relaxed) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(200));
    }
    stream.stop(false)?;
    Ok(())
}
