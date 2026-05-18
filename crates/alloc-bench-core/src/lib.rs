//! Core library for alloc-bench: harness, scenarios, metrics, output schema.

pub mod harness;
pub mod metrics;
pub mod output;
pub mod scenarios;

pub use harness::{run, HarnessConfig, HarnessOutcome, Scenario, SinkValue};
pub use output::SCHEMA_VERSION;
