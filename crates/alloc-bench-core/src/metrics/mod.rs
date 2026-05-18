pub mod env;
pub mod rusage;
pub mod statm;

pub fn allocator_stats_default() -> serde_json::Value {
    serde_json::json!({"kind": "system"})
}
