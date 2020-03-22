use crate::settings::{GetSettingsResponse, SetSettingsResponse, SettingParams};
use sled::Tree;

#[derive(Clone, Default)]
pub struct Settings {}

impl Settings {
    pub fn set(&self, username: String, s: SettingParams, db: Tree) -> SetSettingsResponse {
        match db.insert(
            format!("{}#settings", username),
            serde_json::to_vec(&s).unwrap(),
        ) {
            Ok(_) => SetSettingsResponse {
                status: "success".to_string(),
            },
            Err(e) => SetSettingsResponse {
                status: format!("failed set settings, msg: {}", e.to_string()),
            },
        }
    }
    pub fn get(&self, username: String, db: Tree) -> GetSettingsResponse {
        let result = db.get(format!("{}#settings", username));
        match result {
            Ok(bytes) => GetSettingsResponse {
                settings: match bytes {
                    Some(b) => serde_json::from_slice(&b).ok(),
                    None => None,
                },
                status: "success".to_string(),
            },
            Err(e) => GetSettingsResponse {
                settings: None,
                status: format!("failed get settings, msg: {}", e.to_string()),
            },
        }
    }
}
