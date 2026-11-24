//! Bundles a DMABUF camera input, held between frames, with a faster signal
//! input, and prints each frame's first plane and freshness.

use std::time::{Duration, Instant};

use pipewire_flow::{Filter, PixelFormat, PortDirection, PortMemory, VideoConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = Filter::new("rust-filter-dmabuf-bundle", |buffers| {
        // Buffers come in the order the ports were added: video, then signal.
        let [video, signal] = buffers else { return };
        let signal_bytes = signal.input().map_or(0, <[u8]>::len);
        match video.dmabuf_planes().first() {
            // is_fresh() tells a new frame from a held one, and seq() counts
            // only the new ones.
            Some(plane) => println!(
                "video fd={} stride={} {} seq={} (pts={:?} ns), signal bytes={signal_bytes}",
                plane.fd,
                plane.stride,
                if video.is_fresh() { "new " } else { "held" },
                video.seq(),
                video.pts()
            ),
            None => println!("no DMABUF frame yet, signal bytes={signal_bytes}"),
        }
    })?;

    let video = filter.add_video_port_with(
        PortDirection::Input,
        &VideoConfig::new(640, 480, PixelFormat::Rgb).with_fps(30),
        PortMemory::Dmabuf,
    )?;
    let signal = filter.add_signal_port(PortDirection::Input)?;

    // The camera (about 33 ms a frame) is slower than the signal, so its last
    // frame is held to keep every bundle complete.
    filter.set_port_hold(video, true)?;
    // A hint to bundle no coarser than every 10 ms; the graph still picks
    // the driver.
    filter.set_period_hint(Duration::from_millis(10))?;
    filter.start()?;
    println!("link a DMABUF video source to rust-filter-dmabuf-bundle's video port");

    let mut value = 0.0f32;
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        filter.push_port_data(signal, &value.to_ne_bytes(), None)?;
        value += 0.05;
        if value > 1.0 {
            value -= 2.0;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    filter.stop(false)?;
    Ok(())
}
