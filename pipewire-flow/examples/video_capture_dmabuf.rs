//! Captures from the default camera as DMABUF and prints each frame's first
//! plane instead of its bytes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pipewire_flow::{PixelFormat, PortMemory, Stream, VideoConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = Stream::video_capture(|buf| {
        // A DMABUF frame has no mapped bytes, so data() is None and the
        // planes describe it instead.
        let planes = buf.dmabuf_planes();
        match planes.first() {
            Some(plane) => println!(
                "{} plane(s), fd={} stride={} size={} (pts={:?} ns)",
                planes.len(),
                plane.fd,
                plane.stride,
                plane.size,
                buf.pts()
            ),
            None => println!("frame with no DMABUF plane (pts={:?} ns)", buf.pts()),
        }
    })?;

    // A source that cannot provide DMABUF reports it here, asynchronously.
    let lost = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&lost);
    stream.set_error_callback(move |err| {
        eprintln!("source lost or could not negotiate DMABUF: {err}");
        flag.store(true, Ordering::Relaxed);
    })?;

    stream.set_video_config_with(
        &VideoConfig::new(640, 480, PixelFormat::Yuyv).with_fps(30),
        PortMemory::Dmabuf,
    )?;
    stream.start()?;

    let deadline = Instant::now() + Duration::from_secs(10);
    while !lost.load(Ordering::Relaxed) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(200));
    }
    stream.stop(false)?;
    Ok(())
}
