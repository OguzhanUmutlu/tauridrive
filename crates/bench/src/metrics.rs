use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub duration_secs: f64,
    pub frame_count: usize,
    pub fps: f64,
    pub mean_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub p95_frame_time_ms: f64,
    pub p99_frame_time_ms: f64,
    pub jank_frame_count: usize,
    pub jank_percentage: f64,
    pub js_heap_used_bytes: Option<u64>,
    pub layout_duration_ms: Option<f64>,
    pub recalc_style_duration_ms: Option<f64>,
}
