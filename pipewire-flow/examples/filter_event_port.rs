//! Echoes every event from an event input port back out through an event
//! output port, one cycle at a time.

use std::time::Duration;

use pipewire_flow::{EventKind, Filter, PortDirection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = Filter::new("rust-filter-event-port", |buffers| {
        // Buffers come in the order the ports were added: input, then output.
        let [input, output] = buffers else { return };
        for event in input.events() {
            let kind = match event.kind {
                EventKind::Midi => "midi",
                EventKind::Osc => "osc",
                EventKind::Property => "property",
                _ => "unknown",
            };
            println!(
                "kind={kind} offset={} size={}",
                event.offset,
                event.data.len()
            );
            // A full output buffer refuses the event; the next cycle has room.
            if let Err(err) = output.push_event(&event) {
                eprintln!("dropped an event: {err}");
            }
        }
    })?;

    let input = filter.add_event_port(PortDirection::Input)?;
    let output = filter.add_event_port(PortDirection::Output)?;
    println!(
        "input port is {:?}, output port is {:?}",
        filter.port_data_type(input),
        filter.port_data_type(output)
    );

    filter.start()?;
    println!("echoing events; connect an event source to rust-filter-event-port with pw-link");
    std::thread::sleep(Duration::from_secs(30));
    filter.stop(false)?;
    Ok(())
}
