# tauridrive

> **Lightweight CDP automation, script runner, and visual benchmarking harness for Tauri applications.**

[![Crates.io](https://img.shields.io/badge/crates.io-v0.1.0-orange.svg)](https://crates.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-brightgreen.svg)]()

`tauridrive` is a high-performance native automation driver and visual benchmarking tool built for Tauri apps. Unlike monolithic browser frameworks (Selenium, Playwright/Puppeteer) that bring multi-hundred-megabyte runtimes and webdriver binaries, `tauridrive` communicates directly with your application's webview over a lightweight Chrome DevTools Protocol (CDP) WebSocket connection.

---

## ✨ Key Features

- **🚀 Script Execution**: Run scripts and evaluate JavaScript expressions directly in the webview context with automatic promise awaiting and structured JSON output.
- **🖱️ Precision Input Automation**: Dispatch real mouse moves, hover actions (activating CSS `:hover` states and DOM events without button clicks), clicks, and keyboard sequences.
- **📸 Window-Specific Screenshots (Even Occluded / Background)**: Captures pixel-perfect screenshots directly from the compositor buffer (`Page.captureScreenshot` via surface). **Captures only that specific window**, even if it is behind other windows, unfocused, or placed off-screen.
- **🎬 Animation & Multi-Frame Recording**:
  - **Deterministic Frame Stepping**: Step animation frames deterministically with `HeadlessExperimental.beginFrame` for jitter-free visual testing and video/GIF frame generation.
  - **Live Screencasting**: Stream real-time frames using `Page.startScreencast`.
  - **Animation Domain Control**: Query and adjust playback rates (`Animation.setPlaybackRate`) for slow-motion analysis or fast-forward benchmarking.
- **⚡ Visual Benchmarking Harness**: Measure frame rate (FPS), frame-time distributions (mean, min, max, p95, p99), jank/dropped frame percentages, and engine performance metrics (`LayoutDuration`, `RecalcStyleDuration`, `JSHeapUsedSize`).
- **🪶 Zero Heavyweight Dependencies**: Single standalone native binary written in Rust.

---

## 📦 Architecture

```
tauridrive
├── crates/cdp      - Low-level typed CDP WebSocket client, JSON-RPC, and target discovery
├── crates/driver   - High-level automation: element resolution, input, scripts, and capture
├── crates/bench    - Visual benchmarking harness, jank calculation, and image diffing
└── crates/cli      - Standalone command-line interface (`tauridrive`)
```

---

## 🚀 Quick Start

### Installation

```bash
# Build from source
cargo install --path crates/cli
```

### Launching & Attaching

To automate a Tauri application, ensure remote debugging is enabled:
- **Windows (WebView2)**: Set `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"` or pass `--remote-debugging-port=9222`.
- **Chromium / CEF**: Pass `--remote-debugging-port=9222`.

`tauridrive` handles this automatically with `tauridrive launch`:

```bash
# Launch Tauri application and attach automatically
tauridrive launch ./target/release/my-tauri-app --port 9222
```

Or attach to an already running app:

```bash
tauridrive attach --port 9222
```

---

## 💻 CLI Commands

### 1. Evaluate Code & Run Scripts

```bash
# Evaluate an expression
tauridrive eval "document.title" --port 9222

# Run an automation script
tauridrive run ./examples/smoke_test.js --port 9222
```

### 2. Click, Move & Hover

```bash
# Hover over an element (triggers CSS :hover and DOM mouseover/mouseenter)
tauridrive hover "#nav-dashboard" --port 9222

# Click an element by CSS selector
tauridrive click "#submit-btn" --port 9222

# Right-click or double-click
tauridrive click "#item-row" --button right --port 9222
tauridrive click "#open-file" --count 2 --port 9222

# Move mouse to exact coordinates
tauridrive move 350.5 420.0 --port 9222
```

### 3. Capture Screenshots (Background / Occluded)

Because `tauridrive` queries the rendering engine's compositor surface directly, it captures **only your app's window**, completely unaffected by overlapping desktop windows or focus state.

```bash
# Window screenshot
tauridrive screenshot ./dashboard.png --port 9222

# Specific DOM element screenshot
tauridrive screenshot ./chart.png --selector "#main-chart" --port 9222

# Full page screenshot (beyond viewport)
tauridrive screenshot ./fullpage.png --full-page --port 9222
```

### 4. Record Animation Frames

Record sequential image frames for animation inspection, video compiling, or visual diffs:

```bash
tauridrive record --output-dir ./frames/ --duration 3.0 --fps 60 --port 9222
```

### 5. Visual Benchmarking

Run visual benchmark scenarios and output comprehensive metrics:

```bash
tauridrive bench \
  --script ./examples/bench_animation.js \
  --duration 5.0 \
  --fps-target 60 \
  --report ./benchmark-report.json \
  --frames-dir ./bench-frames/ \
  --port 9222
```

**Example Benchmark Output:**
```
================ BENCHMARK RESULTS ================
  Frames Captured : 300
  Duration        : 5.01s
  Average FPS     : 59.88
  Mean Frame Time : 16.68 ms
  Min Frame Time  : 16.51 ms
  Max Frame Time  : 17.84 ms
  95th Percentile : 16.92 ms
  99th Percentile : 17.20 ms
  Jank Frames     : 0 (0.0%)
  JS Heap Used    : 14.28 MB
  Layout Duration : 1.42 ms
  Recalc Style Dur: 0.85 ms
===================================================
```

---

## 🛠️ Rust Library Usage

You can also use `tauridrive-driver` and `tauridrive-bench` directly in your Rust test suites:

```rust
use std::time::Duration;
use tauridrive_driver::capture::{ScreenshotFormat, ScreenshotOptions};
use tauridrive_driver::Driver;

#[tokio::test]
async fn test_tauri_ui() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Tauri app remote debugging port
    let mut driver = Driver::attach_to_target("127.0.0.1", 9222, Duration::from_secs(5)).await?;

    // Evaluate script
    let title = driver.eval("document.title").await?;
    println!("Title: {}", title);

    // Hover and Click
    driver.hover("#menu-item").await?;
    driver.click("#login-button").await?;

    // Capture occluded window screenshot
    let shot = driver.screenshot(ScreenshotOptions {
        format: ScreenshotFormat::Png,
        from_surface: true,
        ..Default::default()
    }).await?;
    std::fs::write("screenshot.png", shot)?;

    // Step frame deterministically
    let frame = driver.step_frame(16.666, true).await?;
    assert!(frame.is_some());

    Ok(())
}
```

---

## 📄 License

Licensed under the [MIT License](LICENSE).
