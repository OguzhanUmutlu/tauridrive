use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetInfo {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub websocket_debugger_url: Option<String>,
    #[serde(rename = "devtoolsFrontendUrl")]
    pub devtools_frontend_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionInfo {
    #[serde(rename = "Browser")]
    pub browser: Option<String>,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: Option<String>,
    #[serde(rename = "User-Agent")]
    pub user_agent: Option<String>,
    #[serde(rename = "V8-Version")]
    pub v8_version: Option<String>,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub websocket_debugger_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_target_info() {
        let json_data = r#"[
            {
                "description": "",
                "devtoolsFrontendUrl": "/devtools/inspector.html?ws=127.0.0.1:9222/devtools/page/D123",
                "id": "D123",
                "title": "Tauri App Dashboard",
                "type": "page",
                "url": "tauri://localhost/index.html",
                "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/D123"
            }
        ]"#;

        let targets: Vec<TargetInfo> = serde_json::from_str(json_data).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "D123");
        assert_eq!(targets[0].target_type, "page");
        assert_eq!(targets[0].title, "Tauri App Dashboard");
        assert_eq!(
            targets[0].websocket_debugger_url,
            Some("ws://127.0.0.1:9222/devtools/page/D123".to_string())
        );
    }

    #[test]
    fn test_deserialize_version_info() {
        let json_data = r#"{
            "Browser": "Chrome/120.0.6099.109",
            "Protocol-Version": "1.3",
            "User-Agent": "Mozilla/5.0 ...",
            "V8-Version": "12.0.267.10",
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/browser/abc"
        }"#;

        let version: VersionInfo = serde_json::from_str(json_data).unwrap();
        assert_eq!(version.browser, Some("Chrome/120.0.6099.109".to_string()));
        assert_eq!(version.protocol_version, Some("1.3".to_string()));
        assert_eq!(
            version.websocket_debugger_url,
            Some("ws://127.0.0.1:9222/devtools/browser/abc".to_string())
        );
    }
}
