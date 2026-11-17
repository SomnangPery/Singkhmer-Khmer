//! The shared core of the Khmer keyboard.
//!
//! Everything a platform layer needs is re-exported here:
//!
//! * [`normalize()`] puts each Khmer syllable into its one correct Unicode order.
//! * [`validate()`] / [`is_valid()`] check text against the Modern Khmer syllable
//!   structure.
//! * [`Engine`] is what a keyboard talks to: key presses go in, text for the app
//!   comes out.
//! * [`Romanizer`] turns romanized Khmer ("nh jg tv") into Khmer script.
//!
//! The ordering rules follow Unicode Technical Note #61, "Khmer Encoding
//! Structure" (version 2, February 2025). The README lists every decision.

pub mod chars;
mod compose;
mod engine;
mod normalize;
pub mod romanize;
mod validate;

pub use engine::{Engine, Mode, Output, BACKSPACE};
pub use normalize::{normalize, normalize_with, Options};
pub use romanize::Romanizer;
pub use validate::{is_valid, validate, Invalid};
