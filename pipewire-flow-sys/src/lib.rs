//! Raw FFI bindings to [pipewire-flow], a C library that gives PipeWire
//! applications a simpler API for audio and video capture, audio playback,
//! and multi-port filters that bundle every input into one graph cycle.
//!
//! Everything here is generated from the C headers and is `unsafe` to call.
//! Use the `pipewire-flow` crate for a safe interface.
//!
//! [pipewire-flow]: https://github.com/pipewire-flow/pipewire-flow

#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
#![allow(clippy::all)]
// The C headers' Doxygen markers, such as `@param[in]`, read as broken links.
#![allow(rustdoc::broken_intra_doc_links)]
#![no_std]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
