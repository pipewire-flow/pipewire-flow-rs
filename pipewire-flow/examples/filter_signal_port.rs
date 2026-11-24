//! Feeds a synthetic signal port alongside an audio input port into one
//! filter, standing in for a real sensor.

use std::time::{Duration, Instant};

use pipewire_flow::{AudioConfig, Filter, PortDirection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = Filter::new("rust-filter-signal-port", |buffers| {
        // Buffers come in the order the ports were added: signal, then audio.
        let [signal, audio] = buffers else { return };
        let values = signal.input().map_or(0, |data| data.len() / 4);
        let audio_bytes = audio.input().map_or(0, <[u8]>::len);
        println!(
            "signal values={values} (pts={:?} ns) audio bytes={audio_bytes}",
            signal.pts()
        );
    })?;

    let signal = filter.add_signal_port(PortDirection::Input)?;
    filter.add_audio_port(PortDirection::Input, &AudioConfig::new(48_000, 2))?;
    filter.start()?;

    // A ramp wrapping at 1.0. A synthetic value has no hardware timestamp,
    // so none is pushed with it.
    let mut value = 0.0f32;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        filter.push_port_data(signal, &value.to_ne_bytes(), None)?;
        value += 0.05;
        if value > 1.0 {
            value -= 2.0;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    filter.stop(false)?;
    Ok(())
}
