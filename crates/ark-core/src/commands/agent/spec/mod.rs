//! `ark agent spec` — feature-SPEC extraction, registration, and import.

/// Extracts feature SPEC content from deep-tier task plans.
pub mod extract;
/// Imports a feature SPEC authored from an existing codebase.
pub mod import;
/// Registers feature SPEC rows in the feature index.
pub mod register;

pub use extract::{SpecExtractOptions, SpecExtractSummary, spec_extract};
pub use import::{EXTRACTED_SENTINEL, SpecImportOptions, SpecImportSummary, spec_import};
pub use register::{SpecRegisterOptions, SpecRegisterSummary, spec_register};
