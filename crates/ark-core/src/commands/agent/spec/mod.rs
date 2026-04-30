//! `ark agent spec` — feature-SPEC extraction and registration.

/// Extracts feature SPEC content from deep-tier task plans.
pub mod extract;
/// Registers feature SPEC rows in the feature index.
pub mod register;

pub use extract::{SpecExtractOptions, SpecExtractSummary, spec_extract};
pub use register::{SpecRegisterOptions, SpecRegisterSummary, spec_register};
