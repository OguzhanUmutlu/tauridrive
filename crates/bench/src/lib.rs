pub mod diff;
pub mod error;
pub mod harness;
pub mod metrics;

pub use diff::{compare_images, DiffResult};
pub use error::{BenchError, BenchResult};
pub use harness::BenchmarkHarness;
pub use metrics::BenchmarkMetrics;
