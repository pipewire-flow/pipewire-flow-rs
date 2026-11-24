//! Links a camera, and optionally a microphone, straight into a filter by
//! node name, taking the video port's format from the camera.
//!
//! Usage: `filter_port_link <video-node> [audio-node]`. Node names come from
//! `wpctl status` or `pw-cli ls Node`; an object.serial or `node:port` works too.

use std::time::Duration;

use pipewire_flow::{Error, Filter, PixelFormat, PortDirection, PortMemory, VideoConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(video_target) = args.next() else {
        eprintln!("usage: filter_port_link <video-node> [audio-node]");
        std::process::exit(1);
    };
    let audio_target = args.next();

    let mut cycles = 0u64;
    let filter = Filter::new("rust-port-link", move |buffers| {
        // The graph runs far faster than a log, so report only occasionally.
        cycles += 1;
        if cycles % 100 != 1 {
            return;
        }
        for (i, buf) in buffers.iter().enumerate() {
            if let Some(plane) = buf.dmabuf_planes().first() {
                let (fd, stride) = (plane.fd, plane.stride);
                println!(
                    "  port {i}: dmabuf fd={fd} stride={stride} fresh={} seq={}",
                    buf.is_fresh(),
                    buf.seq()
                );
            } else if let Some(data) = buf.input() {
                println!("  port {i}: {} bytes fresh={}", data.len(), buf.is_fresh());
            }
        }
    })?;
    filter.set_error_callback(|_, err| {
        if err == Error::SourceUnavailable {
            println!("a linked source went away; its port is now unlinked");
        }
    })?;

    // A filter port has no converter, so a size the camera lacks would only
    // surface at link time. Asking first avoids the guess.
    let fallback = VideoConfig::new(640, 480, PixelFormat::Yuyv).with_fps(30);
    let formats = filter
        .target_video_formats(&video_target)
        .unwrap_or_default();
    let config = match formats.iter().find_map(|format| format.to_config()) {
        Some(config) => {
            println!(
                "using {config:?}, from the {} format(s) '{video_target}' offers",
                formats.len()
            );
            config
        }
        None => {
            println!("'{video_target}' reported no usable format; trying {fallback:?}");
            fallback
        }
    };

    let video = filter.add_video_port_with(PortDirection::Input, &config, PortMemory::Dmabuf)?;
    let audio = match audio_target {
        Some(_) => Some(filter.add_signal_port(PortDirection::Input)?),
        None => None,
    };
    // Hold re-presents the camera's last frame on cycles it produced
    // nothing, so a faster audio port does not starve the bundle.
    filter.set_port_hold(video, true)?;

    // Linking comes after the start, because the target is looked up in the
    // running graph, where the filter's own node has to exist first.
    filter.start()?;
    report(
        "video",
        &video_target,
        filter.link_port(video, &video_target),
    );
    if let (Some(port), Some(target)) = (audio, &audio_target) {
        report("signal", target, filter.link_port(port, target));
    }

    std::thread::sleep(Duration::from_secs(30));
    // Stopping releases every link on its own; this is only for illustration.
    filter.unlink_port(video)?;
    if let Some(port) = audio {
        filter.unlink_port(port)?;
    }
    filter.stop(false)?;
    Ok(())
}

fn report(port: &str, target: &str, result: Result<(), Error>) {
    let text = match result {
        Ok(()) => "linked",
        Err(Error::NotFound) => "no such target; check `wpctl status` or `pw-cli ls Node`",
        Err(Error::InvalidFormat) => "target found, but the formats do not negotiate",
        Err(Error::InvalidState) => "the port is already linked",
        Err(Error::Timeout) => "the link did not negotiate in time",
        Err(Error::ConnectFailed) => "the link could not be created",
        Err(_) => "the link failed",
    };
    println!("link {port} port -> '{target}': {text}");
}
