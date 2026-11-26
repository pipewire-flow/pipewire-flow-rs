//! Checks that need no PipeWire daemon: pure conversions and error mapping.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use pipewire_flow::{
    log, sys, AudioConfig, DataType, Error, EventKind, PixelFormat, PortDirection, PortMemory,
    SampleFormat, VideoConfig, C_API_VERSION,
};

#[test]
fn c_errors_carry_their_code_and_a_message() {
    let mapped = [
        (Error::InvalidArgument, -1),
        (Error::ConnectFailed, -2),
        (Error::InvalidFormat, -3),
        (Error::NotConfigured, -4),
        (Error::SourceUnavailable, -5),
        (Error::InCallback, -6),
        (Error::NotFound, -7),
        (Error::Timeout, -8),
        (Error::NoMemory, -9),
        (Error::InvalidState, -10),
        (Error::NoSpace, -11),
    ];
    for (error, code) in mapped {
        assert_eq!(error.code(), Some(code));
        assert!(!error.to_string().is_empty());
    }
    assert_eq!(Error::Unknown(-99).code(), Some(-99));
    assert_eq!(
        Error::Unknown(-99).to_string(),
        "Unknown error (pipewire-flow error -99)"
    );
    assert_eq!(Error::CreateFailed.code(), None);
    assert_eq!(Error::InvalidString.code(), None);
}

#[test]
fn format_names_match_the_c_api() {
    assert_eq!(SampleFormat::default(), SampleFormat::S16);
    assert_eq!(SampleFormat::S24In32.as_str(), "S24_32");
    assert_eq!(SampleFormat::F32.as_str(), "F32");
    assert_eq!(PixelFormat::Yuyv.as_str(), "YUYV");
    assert_eq!(PixelFormat::H264.as_str(), "H264");
    assert!(PixelFormat::Mjpg.is_compressed());
    assert!(!PixelFormat::I420.is_compressed());
}

#[test]
fn config_builders_keep_what_they_are_given() {
    let audio = AudioConfig::new(44_100, 1).with_format(SampleFormat::F32);
    assert_eq!(audio.sample_rate, 44_100);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.format, SampleFormat::F32);
    assert_eq!(AudioConfig::new(48_000, 2).format, SampleFormat::S16);

    let video = VideoConfig::new(1280, 720, PixelFormat::Nv12).with_fps(30);
    assert_eq!((video.width, video.height), (1280, 720));
    assert_eq!(video.pixel_format, PixelFormat::Nv12);
    assert_eq!(video.fps, 30);
    assert_eq!(VideoConfig::new(640, 480, PixelFormat::Rgb).fps, 0);
}

#[test]
fn enum_variants_stay_distinct() {
    assert_ne!(DataType::Audio, DataType::Video);
    assert_ne!(DataType::Signal, DataType::Event);
    assert_ne!(PortDirection::Input, PortDirection::Output);
    assert_ne!(EventKind::Midi, EventKind::Property);
    assert_eq!(PortMemory::default(), PortMemory::Auto);
}

#[test]
fn the_bound_c_api_is_the_pinned_one() {
    assert_eq!(C_API_VERSION.0, 0);
    assert_eq!(C_API_VERSION.1, 13);
}

/// Makes the C library log one error, with no daemon involved.
fn log_audio_port_error() {
    let config = sys::pwf_audio_config::default();
    let port = unsafe {
        sys::pwf_filter_add_audio_port(std::ptr::null_mut(), sys::PWF_FILTER_PORT_INPUT, &config)
    };
    assert!(port.is_null());
}

fn log_signal_port_error() {
    let port = unsafe {
        sys::pwf_filter_add_signal_port(std::ptr::null_mut(), sys::PWF_FILTER_PORT_INPUT)
    };
    assert!(port.is_null());
}

/// The log callback is process-wide, so every check on it shares one test.
#[test]
fn the_log_callback_survives_panics_reentry_and_contention() {
    // A panicking callback must leave the next one installable and called.
    log::set_callback(|_| panic!("a log callback that panics"));
    log_audio_port_error();

    let audio = Arc::new(AtomicUsize::new(0));
    let signal = Arc::new(AtomicUsize::new(0));
    let (a, s) = (Arc::clone(&audio), Arc::clone(&signal));
    log::set_callback(move |record| {
        if record
            .message
            .contains("pwf_filter_add_audio_port: filter is NULL")
        {
            a.fetch_add(1, Ordering::Relaxed);
            // A message logged from inside the callback is dropped, not deadlocked on.
            log_signal_port_error();
        } else if record
            .message
            .contains("pwf_filter_add_signal_port: filter is NULL")
        {
            s.fetch_add(1, Ordering::Relaxed);
        }
    });
    log_audio_port_error();
    assert_eq!(
        audio.load(Ordering::Relaxed),
        1,
        "the callback after a panic was not called"
    );
    assert_eq!(
        signal.load(Ordering::Relaxed),
        0,
        "a reentrant message was delivered"
    );

    // Messages from several threads at once are all delivered.
    const PER_THREAD: usize = 200;
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..PER_THREAD {
                    log_audio_port_error();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(audio.load(Ordering::Relaxed), 1 + 4 * PER_THREAD);

    log::clear_callback();
}
