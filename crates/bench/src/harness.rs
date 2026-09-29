use crate::error::BenchResult;
use crate::metrics::BenchmarkMetrics;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};
use tauridrive_driver::capture::ScreenshotFormat;
use tauridrive_driver::Driver;

pub struct BenchmarkHarness<'a> {
    driver: &'a Driver,
    target_fps: f64,
}

impl<'a> BenchmarkHarness<'a> {
    pub fn new(driver: &'a Driver, target_fps: f64) -> Self {
        Self {
            driver,
            target_fps,
        }
    }

    pub async fn run_benchmark(
        &self,
        duration: Duration,
        output_frames_dir: Option<&Path>,
    ) -> BenchResult<BenchmarkMetrics> {
        let client = self.driver.client();
        let _ = client.send_command("Performance.enable", json!({})).await;

        let mut event_rx = client.subscribe();
        self.driver
            .start_screencast(ScreenshotFormat::Png, Some(90))
            .await?;

        let start_time = Instant::now();
        let mut frame_timestamps: Vec<Instant> = Vec::new();
        let mut frame_bytes_vec: Vec<Vec<u8>> = Vec::new();

        let jank_threshold_ms = (1000.0 / self.target_fps) * 1.5;

        if let Some(dir) = output_frames_dir {
            tokio::fs::create_dir_all(dir).await?;
        }

        while start_time.elapsed() < duration {
            tokio::select! {
                Ok(event) = event_rx.recv() => {
                    if event.method == "Page.screencastFrame" {
                        let now = Instant::now();
                        frame_timestamps.push(now);

                        if let Some(session_id) = event.params.get("sessionId").and_then(|s| s.as_u64()) {
                            let _ = self.driver.ack_screencast_frame(session_id as u32).await;
                        }

                        if let Some(b64_data) = event.params.get("data").and_then(|d| d.as_str()) {
                            if let Ok(bytes) = BASE64.decode(b64_data) {
                                if let Some(dir) = output_frames_dir {
                                    let idx = frame_timestamps.len();
                                    let frame_path = dir.join(format!("frame_{:05}.png", idx));
                                    let _ = tokio::fs::write(&frame_path, &bytes).await;
                                }
                                frame_bytes_vec.push(bytes);
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(10)) => {}
            }
        }

        let _ = self.driver.stop_screencast().await;

        // Fetch Performance metrics
        let perf_metrics = client
            .send_command("Performance.getMetrics", json!({}))
            .await
            .ok();

        let mut js_heap_used = None;
        let mut layout_duration = None;
        let mut recalc_style_duration = None;

        if let Some(pm) = perf_metrics {
            if let Some(metrics_arr) = pm.get("metrics").and_then(|m| m.as_array()) {
                for m in metrics_arr {
                    let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let val = m.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    match name {
                        "JSHeapUsedSize" => js_heap_used = Some(val as u64),
                        "LayoutDuration" => layout_duration = Some(val * 1000.0), // ms
                        "RecalcStyleDuration" => recalc_style_duration = Some(val * 1000.0), // ms
                        _ => {}
                    }
                }
            }
        }

        let total_duration_secs = start_time.elapsed().as_secs_f64();
        let frame_count = frame_timestamps.len();

        let mut frame_deltas: Vec<f64> = Vec::new();
        for i in 1..frame_timestamps.len() {
            let delta = frame_timestamps[i]
                .duration_since(frame_timestamps[i - 1])
                .as_secs_f64()
                * 1000.0;
            frame_deltas.push(delta);
        }

        let fps = if total_duration_secs > 0.0 {
            frame_count as f64 / total_duration_secs
        } else {
            0.0
        };

        let (mean_frame_time, min_frame_time, max_frame_time, p95_frame_time, p99_frame_time, jank_count) =
            if !frame_deltas.is_empty() {
                let sum: f64 = frame_deltas.iter().sum();
                let mean = sum / frame_deltas.len() as f64;
                let min = frame_deltas
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);
                let max = frame_deltas
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max);

                let mut sorted = frame_deltas.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let p95_idx = ((sorted.len() as f64 * 0.95).floor() as usize).min(sorted.len() - 1);
                let p99_idx = ((sorted.len() as f64 * 0.99).floor() as usize).min(sorted.len() - 1);

                let jank = frame_deltas
                    .iter()
                    .filter(|&&d| d > jank_threshold_ms)
                    .count();

                (mean, min, max, sorted[p95_idx], sorted[p99_idx], jank)
            } else {
                (0.0, 0.0, 0.0, 0.0, 0.0, 0)
            };

        let jank_pct = if !frame_deltas.is_empty() {
            (jank_count as f64 / frame_deltas.len() as f64) * 100.0
        } else {
            0.0
        };

        Ok(BenchmarkMetrics {
            duration_secs: total_duration_secs,
            frame_count,
            fps,
            mean_frame_time_ms: mean_frame_time,
            min_frame_time_ms: min_frame_time,
            max_frame_time_ms: max_frame_time,
            p95_frame_time_ms: p95_frame_time,
            p99_frame_time_ms: p99_frame_time,
            jank_frame_count: jank_count,
            jank_percentage: jank_pct,
            js_heap_used_bytes: js_heap_used,
            layout_duration_ms: layout_duration,
            recalc_style_duration_ms: recalc_style_duration,
        })
    }
}
