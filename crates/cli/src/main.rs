use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;
use tauridrive_bench::BenchmarkHarness;
use tauridrive_driver::capture::{ScreenshotFormat, ScreenshotOptions};
use tauridrive_driver::input::MouseButton;
use tauridrive_driver::Driver;

#[derive(Parser, Debug)]
#[command(name = "tauridrive")]
#[command(about = "Lightweight CDP automation, script runner, and visual benchmarking harness for Tauri", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "127.0.0.1", global = true)]
    host: String,

    #[arg(short, long, default_value_t = 9222, global = true)]
    port: u16,

    /// Timeout in seconds to wait for CDP targets to become ready
    #[arg(long, default_value_t = 15, global = true)]
    timeout: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch a Tauri application with CDP enabled and attach to it
    Launch {
        /// Path to the Tauri application executable
        app: PathBuf,

        /// Timeout in seconds to wait for application to start
        #[arg(long, default_value_t = 30)]
        timeout: u64,

        /// Additional arguments to pass to the Tauri application
        #[arg(last = true)]
        app_args: Vec<String>,
    },

    /// Attach to an already running Tauri app's CDP port and inspect info
    Attach,

    /// Evaluate a JavaScript expression inside the Tauri webview
    Eval {
        /// JavaScript expression to evaluate
        expression: String,
    },

    /// Run a JavaScript file inside the Tauri webview
    Run {
        /// Path to the JavaScript file
        script: PathBuf,
    },

    /// Click an element identified by a CSS selector
    Click {
        /// CSS selector of the target element
        selector: String,

        /// Mouse button: left, right, middle
        #[arg(long, default_value = "left")]
        button: String,

        /// Number of clicks (1 for single, 2 for double click)
        #[arg(long, default_value_t = 1)]
        count: i32,
    },

    /// Hover over an element identified by a CSS selector
    Hover {
        /// CSS selector of the target element
        selector: String,
    },

    /// Move mouse cursor to specific coordinates
    Move {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },

    /// Capture a screenshot (works even if window is occluded or in background)
    Screenshot {
        /// Output path for the image (e.g. shot.png)
        output: PathBuf,

        /// CSS selector to clip screenshot to a specific DOM element
        #[arg(short, long)]
        selector: Option<String>,

        /// Capture beyond viewport (full scrollable height)
        #[arg(long)]
        full_page: bool,
    },

    /// Record sequential animation frames
    Record {
        /// Directory to output frames
        #[arg(short, long)]
        output_dir: PathBuf,

        /// Duration to record in seconds
        #[arg(short, long, default_value_t = 5.0)]
        duration: f64,

        /// Target frame rate
        #[arg(long, default_value_t = 60.0)]
        fps: f64,
    },

    /// Run visual and performance benchmark
    Bench {
        /// Duration of benchmark in seconds
        #[arg(short, long, default_value_t = 5.0)]
        duration: f64,

        /// Target frame rate
        #[arg(long, default_value_t = 60.0)]
        fps_target: f64,

        /// Optional JavaScript file to run during benchmark
        #[arg(short, long)]
        script: Option<PathBuf>,

        /// Path to save JSON report
        #[arg(short, long)]
        report: Option<PathBuf>,

        /// Optional directory to save captured frames
        #[arg(long)]
        frames_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Launch {
            app,
            timeout,
            app_args,
        } => {
            println!(
                "🚀 Launching {:?} with remote debugging on {}:{}...",
                app, cli.host, cli.port
            );

            let port_arg = format!("--remote-debugging-port={}", cli.port);
            let webview2_args = format!("--remote-debugging-port={}", cli.port);

            let mut cmd = Command::new(&app);
            cmd.arg(&port_arg);
            cmd.args(&app_args);
            cmd.env("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", webview2_args);

            let mut child: Child = cmd
                .spawn()
                .with_context(|| format!("Failed to spawn executable {:?}", app))?;

            println!("⏳ Waiting for webview CDP target to be ready...");
            match Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(timeout)).await {
                Ok(driver) => {
                    println!(" Connected to Tauri webview successfully!");
                    if let Ok(title) = driver.eval("document.title").await {
                        println!("📄 Window Title: {}", title);
                    }
                    if let Ok(url) = driver.eval("window.location.href").await {
                        println!("🔗 Window URL: {}", url);
                    }
                    println!("App is running in background. Press Ctrl+C to terminate.");
                    tokio::signal::ctrl_c().await?;
                    println!("Terminating app...");
                    let _ = child.kill();
                }
                Err(e) => {
                    let _ = child.kill();
                    return Err(anyhow::anyhow!("Failed to connect to target: {}", e));
                }
            }
        }

        Commands::Attach => {
            println!("Connecting to Tauri webview at {}:{}...", cli.host, cli.port);
            let driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;
            println!(" Connected successfully!");
            let title = driver.eval("document.title").await?;
            let url = driver.eval("window.location.href").await?;
            println!("📄 Title: {}", title);
            println!("🔗 URL: {}", url);
        }

        Commands::Eval { expression } => {
            let driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;
            let res = driver.eval(&expression).await?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }

        Commands::Run { script } => {
            let driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;
            println!("Running script {:?}...", script);
            let res = driver.eval_file(&script).await?;
            println!("Result: {}", serde_json::to_string_pretty(&res)?);
        }

        Commands::Click {
            selector,
            button,
            count,
        } => {
            let mut driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;
            let btn = match button.to_lowercase().as_str() {
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            let rect = driver.get_element_rect(&selector).await?;
            let (cx, cy) = rect.center();
            println!(
                "Clicking element '{}' at ({:.1}, {:.1}) [btn: {:?}, count: {}]...",
                selector, cx, cy, btn, count
            );
            driver.click_at(cx, cy, btn, count).await?;
            println!(" Click sent!");
        }

        Commands::Hover { selector } => {
            let mut driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;
            let rect = driver.get_element_rect(&selector).await?;
            let (cx, cy) = rect.center();
            println!("Hovering over '{}' at ({:.1}, {:.1})...", selector, cx, cy);
            driver.hover(&selector).await?;
            println!(" Hover sent!");
        }

        Commands::Move { x, y } => {
            let mut driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;
            println!("Moving cursor to ({:.1}, {:.1})...", x, y);
            driver.move_to(x, y).await?;
            println!(" Mouse moved!");
        }

        Commands::Screenshot {
            output,
            selector,
            full_page,
        } => {
            let driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;

            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_lowercase();
            let format = match ext.as_str() {
                "jpg" | "jpeg" => ScreenshotFormat::Jpeg,
                "webp" => ScreenshotFormat::Webp,
                _ => ScreenshotFormat::Png,
            };

            let bytes = if let Some(ref sel) = selector {
                println!("📸 Capturing element screenshot of '{}'...", sel);
                driver.screenshot_element(sel).await?
            } else {
                println!(
                    "📸 Capturing window screenshot (even if occluded/background, full_page: {})...",
                    full_page
                );
                let options = ScreenshotOptions {
                    format,
                    quality: Some(95),
                    clip: None,
                    from_surface: true,
                    capture_beyond_viewport: full_page,
                };
                driver.screenshot(options).await?
            };

            if let Some(parent) = output.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&output, &bytes).await?;
            println!(" Screenshot saved to {:?}", output);
        }

        Commands::Record {
            output_dir,
            duration,
            fps,
        } => {
            let driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;

            println!(
                "🎬 Recording animation frames for {:.1}s at target {:.0} FPS to {:?}...",
                duration, fps, output_dir
            );

            let harness = BenchmarkHarness::new(&driver, fps);
            let metrics = harness
                .run_benchmark(Duration::from_secs_f64(duration), Some(&output_dir))
                .await?;

            println!(
                " Recorded {} frames ({:.1} FPS actual) in {:.2}s!",
                metrics.frame_count, metrics.fps, metrics.duration_secs
            );
        }

        Commands::Bench {
            duration,
            fps_target,
            script,
            report,
            frames_dir,
        } => {
            let driver = Driver::attach_to_target(&cli.host, cli.port, Duration::from_secs(cli.timeout))
                .await
                .context("Failed to attach to target")?;

            if let Some(ref script_path) = script {
                println!("Executing scenario script {:?}...", script_path);
                driver.eval_file(script_path).await?;
            }

            println!(
                "⚡ Benchmarking visuals for {:.1}s (Target: {:.0} FPS)...",
                duration, fps_target
            );

            let harness = BenchmarkHarness::new(&driver, fps_target);
            let metrics = harness
                .run_benchmark(
                    Duration::from_secs_f64(duration),
                    frames_dir.as_deref(),
                )
                .await?;

            println!("\n================ BENCHMARK RESULTS ================");
            println!("  Frames Captured : {}", metrics.frame_count);
            println!("  Duration        : {:.2}s", metrics.duration_secs);
            println!("  Average FPS     : {:.2}", metrics.fps);
            println!("  Mean Frame Time : {:.2} ms", metrics.mean_frame_time_ms);
            println!("  Min Frame Time  : {:.2} ms", metrics.min_frame_time_ms);
            println!("  Max Frame Time  : {:.2} ms", metrics.max_frame_time_ms);
            println!("  95th Percentile : {:.2} ms", metrics.p95_frame_time_ms);
            println!("  99th Percentile : {:.2} ms", metrics.p99_frame_time_ms);
            println!("  Jank Frames     : {} ({:.1}%)", metrics.jank_frame_count, metrics.jank_percentage);
            if let Some(heap) = metrics.js_heap_used_bytes {
                println!("  JS Heap Used    : {:.2} MB", heap as f64 / (1024.0 * 1024.0));
            }
            if let Some(layout) = metrics.layout_duration_ms {
                println!("  Layout Duration : {:.2} ms", layout);
            }
            if let Some(recalc) = metrics.recalc_style_duration_ms {
                println!("  Recalc Style Dur: {:.2} ms", recalc);
            }
            println!("===================================================\n");

            if let Some(report_path) = report {
                let json = serde_json::to_string_pretty(&metrics)?;
                if let Some(parent) = report_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&report_path, json).await?;
                println!("💾 Report saved to {:?}", report_path);
            }
        }
    }

    Ok(())
}
