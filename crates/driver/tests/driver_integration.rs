use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use tauridrive_driver::capture::{ScreenshotFormat, ScreenshotOptions};
use tauridrive_driver::input::MouseButton;
use tauridrive_driver::Driver;

const SAMPLE_1X1_PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

async fn spawn_mock_cdp_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let ws_url = format!("ws://{}", addr);

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut ws = accept_async(stream).await.unwrap();
                while let Some(Ok(msg)) = ws.next().await {
                    if let Message::Text(text) = msg {
                        let req: Value = serde_json::from_str(&text).unwrap();
                        let id = req["id"].as_u64().unwrap();
                        let method = req["method"].as_str().unwrap();

                        let res = match method {
                            "Page.enable" | "Runtime.enable" | "DOM.enable" | "Animation.enable" => {
                                json!({ "id": id, "result": {} })
                            }
                            "Runtime.evaluate" => {
                                let expr = req["params"]["expression"].as_str().unwrap_or("");
                                if expr.contains("document.querySelector") {
                                    json!({
                                        "id": id,
                                        "result": {
                                            "result": {
                                                "value": {
                                                    "x": 100.0,
                                                    "y": 150.0,
                                                    "width": 80.0,
                                                    "height": 40.0
                                                }
                                            }
                                        }
                                    })
                                } else if expr.contains("document.title") {
                                    json!({
                                        "id": id,
                                        "result": {
                                            "result": {
                                                "value": "Tauri App"
                                            }
                                        }
                                    })
                                } else {
                                    json!({
                                        "id": id,
                                        "result": {
                                            "result": {
                                                "value": 42
                                            }
                                        }
                                    })
                                }
                            }
                            "Input.dispatchMouseEvent" | "Input.dispatchKeyEvent" => {
                                json!({ "id": id, "result": {} })
                            }
                            "Page.captureScreenshot" => {
                                json!({
                                    "id": id,
                                    "result": {
                                        "data": SAMPLE_1X1_PNG_BASE64
                                    }
                                })
                            }
                            "HeadlessExperimental.beginFrame" => {
                                json!({
                                    "id": id,
                                    "result": {
                                        "hasDamage": true,
                                        "screenshotData": SAMPLE_1X1_PNG_BASE64
                                    }
                                })
                            }
                            "Animation.setPlaybackRate" => {
                                json!({ "id": id, "result": {} })
                            }
                            _ => json!({ "id": id, "result": {} }),
                        };

                        let _ = ws.send(Message::Text(res.to_string().into())).await;
                    }
                }
            });
        }
    });

    ws_url
}

#[tokio::test]
async fn test_driver_eval_and_scripts() {
    let ws_url = spawn_mock_cdp_server().await;
    let driver = Driver::connect(&ws_url).await.unwrap();

    let res = driver.eval("21 * 2").await.unwrap();
    assert_eq!(res, json!(42));

    let title = driver.eval("document.title").await.unwrap();
    assert_eq!(title, json!("Tauri App"));
}

#[tokio::test]
async fn test_driver_mouse_click_move_hover() {
    let ws_url = spawn_mock_cdp_server().await;
    let mut driver = Driver::connect(&ws_url).await.unwrap();

    // Test element rect resolution
    let rect = driver.get_element_rect("#btn").await.unwrap();
    assert_eq!(rect.x, 100.0);
    assert_eq!(rect.y, 150.0);
    assert_eq!(rect.width, 80.0);
    assert_eq!(rect.height, 40.0);

    // Test hover
    driver.hover("#btn").await.unwrap();

    // Test click
    driver.click("#btn").await.unwrap();

    // Test click_at
    driver.click_at(140.0, 170.0, MouseButton::Right, 1).await.unwrap();

    // Test move_to
    driver.move_to(300.0, 400.0).await.unwrap();
}

#[tokio::test]
async fn test_driver_keyboard_dispatch() {
    let ws_url = spawn_mock_cdp_server().await;
    let driver = Driver::connect(&ws_url).await.unwrap();

    driver.type_text("Hello Tauri").await.unwrap();
    driver.press_key("Enter").await.unwrap();
}

#[tokio::test]
async fn test_driver_screenshot_occluded_window() {
    let ws_url = spawn_mock_cdp_server().await;
    let driver = Driver::connect(&ws_url).await.unwrap();

    // Capture screenshot from surface (even if window is not on top)
    let options = ScreenshotOptions {
        format: ScreenshotFormat::Png,
        from_surface: true,
        ..Default::default()
    };
    let bytes = driver.screenshot(options).await.unwrap();
    assert!(!bytes.is_empty());

    // Verify it's a valid PNG
    let img = image::load_from_memory(&bytes).unwrap();
    assert_eq!(img.width(), 1);
    assert_eq!(img.height(), 1);
}

#[tokio::test]
async fn test_driver_element_screenshot() {
    let ws_url = spawn_mock_cdp_server().await;
    let driver = Driver::connect(&ws_url).await.unwrap();

    let bytes = driver.screenshot_element("#btn").await.unwrap();
    assert!(!bytes.is_empty());
}

#[tokio::test]
async fn test_driver_deterministic_animation_frame_step() {
    let ws_url = spawn_mock_cdp_server().await;
    let driver = Driver::connect(&ws_url).await.unwrap();

    // Step animation frame by 16.666 ms with screenshot
    let frame_opt = driver.step_frame(16.666, true).await.unwrap();
    assert!(frame_opt.is_some());
    let bytes = frame_opt.unwrap();
    let img = image::load_from_memory(&bytes).unwrap();
    assert_eq!(img.width(), 1);
    assert_eq!(img.height(), 1);
}

#[tokio::test]
async fn test_driver_animation_playback_rate() {
    let ws_url = spawn_mock_cdp_server().await;
    let driver = Driver::connect(&ws_url).await.unwrap();

    driver.set_animation_playback_rate(0.25).await.unwrap();
}
