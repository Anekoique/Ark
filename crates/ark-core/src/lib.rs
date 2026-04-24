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
    ConflictChoice, ConflictPolicy, InitOptions, InitSummary, LoadOptions, LoadSummary, Prompter,
    RemoveOptions, RemoveSummary, UnloadOptions, UnloadSummary, UpgradeOptions, UpgradeSummary,
    agent::{
        Phase, Status, TaskToml, Tier,
        spec::{
            SpecExtractOptions, SpecExtractSummary, SpecRegisterOptions, SpecRegisterSummary,
            spec_extract, spec_register,
        },
        task::{
            TaskArchiveOptions, TaskArchiveSummary, TaskNewOptions, TaskNewSummary,
            TaskPhaseOptions, TaskPhaseSummary, TaskPromoteOptions, TaskPromoteSummary,
            task_archive, task_execute, task_new, task_plan, task_promote, task_review,
            task_verify,
        },
    },
    init, load, remove, unload, upgrade,
};
pub use error::{Error, Result};
pub use io::{PathExt, WriteMode, hash_bytes};
pub use layout::Layout;
