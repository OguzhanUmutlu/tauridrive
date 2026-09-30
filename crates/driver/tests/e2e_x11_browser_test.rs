use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use tauridrive_bench::BenchmarkHarness;
use tauridrive_driver::capture::{ScreenshotFormat, ScreenshotOptions};
use tauridrive_driver::input::MouseButton;
use tauridrive_driver::Driver;

struct ChromeRunner {
    child: Child,
    port: u16,
    profile_dir: PathBuf,
}

impl ChromeRunner {
    fn spawn() -> Option<Self> {
        let candidates = [
            "/usr/bin/google-chrome-stable",
            "/usr/bin/google-chrome",
            "/opt/google/chrome/chrome",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
        ];

        let bin = candidates.iter().find(|p| Path::new(p).exists())?;

        // Find available port
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").ok()?;
            listener.local_addr().ok()?.port()
        };

        let temp_id = format!("tauridrive-test-{}", port);
        let profile_dir = std::env::temp_dir().join(temp_id);
        let _ = std::fs::create_dir_all(&profile_dir);

        let test_html = r#"<!DOCTYPE html>
<html>
<head>
  <title>TauriDrive X11 Live Test</title>
  <style>
    body { margin: 0; padding: 30px; font-family: sans-serif; background: #fafafa; }
    #counter-display { font-size: 24px; font-weight: bold; margin-bottom: 15px; color: #1e293b; }
    #click-btn { padding: 12px 24px; font-size: 16px; background: #3b82f6; color: white; border: none; border-radius: 6px; cursor: pointer; }
    #hover-box { width: 140px; height: 140px; background: #ef4444; border-radius: 10px; margin-top: 20px; transition: background 0.1s; }
    #hover-box:hover { background: #10b981 !important; }
    #text-input { margin-top: 15px; padding: 10px; font-size: 16px; border: 1px solid #cbd5e1; border-radius: 6px; width: 250px; }
  </style>
</head>
<body>
  <div id="counter-display">Clicks: 0</div>
  <button id="click-btn" onclick="
    window.clickCount = (window.clickCount || 0) + 1;
    document.getElementById('counter-display').innerText = 'Clicks: ' + window.clickCount;
  ">Click Me</button>
  <div id="hover-box"></div>
  <div><input id="text-input" placeholder="Type here..." /></div>
</body>
</html>"#;

        let html_file = profile_dir.join("test.html");
        std::fs::write(&html_file, test_html).ok()?;
        let file_url = format!("file://{}", html_file.display());

        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":99".to_string());

        let mut cmd = Command::new(bin);
        cmd.env("DISPLAY", display)
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg(format!("--remote-debugging-port={}", port))
            .arg(format!("--user-data-dir={}", profile_dir.display()))
            .arg("--headless=new")
            .arg(&file_url);

        let child = cmd.spawn().ok()?;
        let runner = Self {
            child,
            port,
            profile_dir,
        };

        // Wait for CDP readiness
        let start = Instant::now();
        let timeout = Duration::from_secs(15);
        let client = reqwest::Client::new();
        let version_url = format!("http://127.0.0.1:{}/json/version", port);

        let mut ready = false;
        while start.elapsed() < timeout {
            let res = std::thread::spawn({
                let url = version_url.clone();
                let client = client.clone();
                move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async { client.get(&url).send().await.is_ok() })
                }
            })
            .join();

            if let Ok(true) = res {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        if !ready {
            return None;
        }

        Some(runner)
    }
}

impl Drop for ChromeRunner {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.profile_dir);
    }
}

#[tokio::test]
async fn test_x11_browser_e2e_script_and_input() {
    let runner = match ChromeRunner::spawn() {
        Some(r) => r,
        None => {
            eprintln!("Skipping test: Chrome binary not found or failed to start.");
            return;
        }
    };

    let mut driver = Driver::attach_to_target("127.0.0.1", runner.port, Duration::from_secs(12))
        .await
        .expect("Failed to attach to Chrome via CDP");

    // Wait for the button to appear in the DOM
    driver
        .wait_for_selector("#click-btn", Duration::from_secs(8))
        .await
        .expect("Element #click-btn did not appear in DOM");

    // 1. Verify Page Title
    let title = driver.eval("document.title").await.unwrap();
    assert_eq!(title, "TauriDrive X11 Live Test");

    // 2. Click button and verify DOM state change
    let initial_text = driver
        .eval("document.getElementById('counter-display').innerText")
        .await
        .unwrap();
    assert_eq!(initial_text, "Clicks: 0");

    driver.click("#click-btn").await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;

    let updated_text = driver
        .eval("document.getElementById('counter-display').innerText")
        .await
        .unwrap();
    assert_eq!(updated_text, "Clicks: 1");

    // Double click button
    driver.click_at(100.0, 50.0, MouseButton::Left, 2).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 3. Hover over element
    driver.hover("#hover-box").await.unwrap();

    // 4. Keyboard focus and typing
    driver.click("#text-input").await.unwrap();
    driver.type_text("Tauri Automation").await.unwrap();

    let input_value = driver
        .eval("document.getElementById('text-input').value")
        .await
        .unwrap();
    assert_eq!(input_value, "Tauri Automation");
}

#[tokio::test]
async fn test_x11_browser_occluded_window_and_element_screenshots() {
    let runner = match ChromeRunner::spawn() {
        Some(r) => r,
        None => {
            eprintln!("Skipping test: Chrome binary not found or failed to start.");
            return;
        }
    };

    let driver = Driver::attach_to_target("127.0.0.1", runner.port, Duration::from_secs(12))
        .await
        .expect("Failed to attach to Chrome via CDP");

    // Wait for the button to appear in the DOM
    driver
        .wait_for_selector("#click-btn", Duration::from_secs(8))
        .await
        .expect("Element #click-btn did not appear in DOM");

    // 1. Capture full window screenshot from surface (even if not on top / occluded)
    let options = ScreenshotOptions {
        format: ScreenshotFormat::Png,
        from_surface: true,
        capture_beyond_viewport: false,
        ..Default::default()
    };
    let window_bytes = driver.screenshot(options).await.unwrap();
    assert!(!window_bytes.is_empty());

    let window_img = image::load_from_memory(&window_bytes).unwrap();
    assert!(window_img.width() > 100);
    assert!(window_img.height() > 100);

    // 2. Capture element screenshot (#click-btn)
    let elem_bytes = driver.screenshot_element("#click-btn").await.unwrap();
    assert!(!elem_bytes.is_empty());

    let elem_img = image::load_from_memory(&elem_bytes).unwrap();
    assert!(elem_img.width() > 30);
    assert!(elem_img.height() > 15);
    // Element screenshot should be smaller than whole window
    assert!(elem_img.width() < window_img.width());
    assert!(elem_img.height() < window_img.height());
}

#[tokio::test]
async fn test_x11_browser_visual_benchmark_harness() {
    let runner = match ChromeRunner::spawn() {
        Some(r) => r,
        None => {
            eprintln!("Skipping test: Chrome binary not found or failed to start.");
            return;
        }
    };

    let driver = Driver::attach_to_target("127.0.0.1", runner.port, Duration::from_secs(12))
        .await
        .expect("Failed to attach to Chrome via CDP");

    let harness = BenchmarkHarness::new(&driver, 60.0);
    let temp_frames_dir = std::env::temp_dir().join(format!("tauridrive-frames-{}", runner.port));

    let metrics = harness
        .run_benchmark(Duration::from_millis(800), Some(&temp_frames_dir))
        .await
        .unwrap();

    assert!(metrics.duration_secs > 0.0);
    // Verify performance metrics are populated
    assert!(metrics.js_heap_used_bytes.is_some());
    assert!(metrics.layout_duration_ms.is_some());
    assert!(metrics.recalc_style_duration_ms.is_some());

    let _ = std::fs::remove_dir_all(&temp_frames_dir);
}
