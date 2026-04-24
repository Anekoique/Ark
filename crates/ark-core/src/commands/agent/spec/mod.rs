//! `ark agent spec` — feature-SPEC extraction and registration.

pub mod extract;
pub mod register;

pub use extract::{SpecExtractOptions, SpecExtractSummary, spec_extract};
pub use register::{SpecRegisterOptions, SpecRegisterSummary, spec_register};
