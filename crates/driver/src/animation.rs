use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationInfo {
    pub id: String,
    pub name: String,
    pub paused_state: bool,
    pub play_state: String,
    pub playback_rate: f64,
    pub start_time: f64,
    pub current_time: f64,
}
