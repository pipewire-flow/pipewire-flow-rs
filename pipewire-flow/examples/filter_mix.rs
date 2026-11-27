//! Mixes two audio input ports into one audio output port.

use std::time::Duration;

use pipewire_flow::{AudioConfig, Filter, PortDirection, SampleFormat};

const SAMPLE: usize = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = Filter::new("rust-mixer", |buffers| {
        // Buffers come in the order the ports were added: two inputs, then
        // the output.
        let [a, b, out] = buffers else { return };
        let (left, right) = (a.input().unwrap_or(&[]), b.input().unwrap_or(&[]));
        let Some(region) = out.output() else { return };

        // The sum goes straight into the output buffer, since this thread
        // must not allocate.
        let len = left.len().max(right.len()).min(region.len()) / SAMPLE * SAMPLE;
        for (i, dst) in region[..len].chunks_exact_mut(SAMPLE).enumerate() {
            let sum = sample(left, i) + sample(right, i);
            dst.copy_from_slice(&sum.to_le_bytes());
        }
        out.set_filled(len);
    })?;

    let config = AudioConfig::new(48_000, 2).with_format(SampleFormat::F32);
    filter.add_audio_port(PortDirection::Input, &config)?;
    filter.add_audio_port(PortDirection::Input, &config)?;
    filter.add_audio_port(PortDirection::Output, &config)?;

    filter.start()?;
    println!("mixing; connect two sources to rust-mixer with pw-link");
    std::thread::sleep(Duration::from_secs(30));
    filter.stop(false)?;
    Ok(())
}

/// The `i`th sample of `bytes`, or silence past its end.
fn sample(bytes: &[u8], i: usize) -> f32 {
    bytes.get(i * SAMPLE..(i + 1) * SAMPLE).map_or(0.0, |b| {
        f32::from_le_bytes(b.try_into().expect("four bytes"))
    })
}
