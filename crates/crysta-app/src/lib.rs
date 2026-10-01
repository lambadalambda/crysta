//! The Crysta slice's host-independent half: the session, its renderer and
//! its music. The native window (`main.rs`) and the web page share it.

// A library for the app's own two hosts, not a published API.
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

pub mod background;
pub mod clock;
pub mod frame;
pub mod mode7;
pub mod music;
pub mod music_data;
pub mod records;
pub mod restart;
pub mod session;
pub mod shop;
pub mod sram_file;
pub mod title;
pub mod window;
