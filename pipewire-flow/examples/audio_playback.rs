//! Plays a 440 Hz tone to the default output device, or to a sink named on
//! the command line.

use std::f32::consts::TAU;
use std::time::Duration;

use pipewire_flow::{AudioConfig, Routing, SampleFormat, Stream};

const RATE: u32 = 48_000;
const CHANNELS: u32 = 2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut phase = 0.0f32;
    let step = TAU * 440.0 / RATE as f32;

    let stream = Stream::playback(move |buf| {
        // Samples go straight into the graph's buffer, since this thread must
        // not allocate.
        let region = buf.as_mut_slice();
        let frame_bytes = CHANNELS as usize * 4;
        let mut filled = 0;
        for frame in region.chunks_exact_mut(frame_bytes) {
            let value = (phase.sin() * 0.2).to_le_bytes();
            phase = (phase + step) % TAU;
            for sample in frame.chunks_exact_mut(4) {
                sample.copy_from_slice(&value);
            }
            filled += frame_bytes;
        }
        buf.set_filled(filled);
    })?;

    if let Some(sink) = std::env::args().nth(1) {
        stream.set_routing(Routing::Autoconnect(Some(&sink)))?;
    }
    stream.set_audio_config(&AudioConfig::new(RATE, CHANNELS).with_format(SampleFormat::F32))?;
    stream.start()?;
    std::thread::sleep(Duration::from_secs(5));
    stream.stop(true)?;
    Ok(())
}
