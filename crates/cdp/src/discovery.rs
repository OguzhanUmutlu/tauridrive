use crate::error::{CdpError, CdpResult};
use crate::protocol::{TargetInfo, VersionInfo};
use std::time::Duration;

pub struct Discovery {
    base_url: String,
    http_client: reqwest::Client,
}

impl Discovery {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            base_url: format!("http://{}:{}", host, port),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn get_version(&self) -> CdpResult<VersionInfo> {
        let url = format!("{}/json/version", self.base_url);
        let res = self.http_client.get(&url).send().await?.json::<VersionInfo>().await?;
        Ok(res)
    }

    pub async fn get_targets(&self) -> CdpResult<Vec<TargetInfo>> {
        let url = format!("{}/json/list", self.base_url);
        let res = self.http_client.get(&url).send().await?.json::<Vec<TargetInfo>>().await?;
        Ok(res)
    }

    pub async fn wait_for_target(&self, timeout: Duration) -> CdpResult<TargetInfo> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(150);

        while start.elapsed() < timeout {
            if let Ok(targets) = self.get_targets().await {
                let valid_targets: Vec<TargetInfo> = targets
                    .into_iter()
                    .filter(|t| t.websocket_debugger_url.is_some())
                    .collect();

                // 1. Prefer explicit page or webview targets
                if let Some(target) = valid_targets
                    .iter()
                    .find(|t| t.target_type == "page" || t.target_type == "webview")
                {
                    return Ok(target.clone());
                }

                // 2. Otherwise prefer anything that is not internal browser UI
                if let Some(target) = valid_targets
                    .iter()
                    .find(|t| t.target_type != "browser" && t.target_type != "browser_ui" && t.target_type != "other")
                {
                    return Ok(target.clone());
                }

                // 3. Fallback to first available target
                if let Some(target) = valid_targets.first() {
                    return Ok(target.clone());
                }
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(CdpError::Timeout(format!(
            "Timed out waiting for Tauri webview target at {}",
            self.base_url
        )))
    }
}
