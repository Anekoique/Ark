//! `ark-core` — library that drives the `ark` CLI.
//!
//! The CLI is a thin shell over this crate. All scaffolding, file writing,
//! snapshotting, and manifest tracking lives here.

pub mod commands;
pub mod error;
pub mod io;
pub mod layout;
pub mod state;
pub mod templates;

pub use commands::{
    InitOptions, InitSummary, LoadOptions, LoadSummary, RemoveOptions, RemoveSummary,
    UnloadOptions, UnloadSummary, init, load, remove, unload,
};
pub use error::{Error, Result};
pub use io::{PathExt, WriteMode};
pub use layout::Layout;
