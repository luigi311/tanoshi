use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::BTreeMap;
use std::iter;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Config {
    pub base_url: Option<String>,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_tls_port")]
    pub tls_port: u16,
    #[serde(default = "String::default")]
    pub cert: String,
    #[serde(default = "String::default")]
    pub key: String,
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_secret")]
    pub secret: String,
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
    pub telegram_token: Option<String>,
    #[serde(default = "default_plugin_path")]
    pub plugin_path: String,
    #[serde(default = "BTreeMap::new")]
    pub plugin_config: BTreeMap<String, serde_yaml::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: None,
            port: default_port(),
            tls_port: default_tls_port(),
            cert: "".to_string(),
            key: "".to_string(),
            database_path: default_database_path(),
            secret: default_secret(),
            update_interval: default_update_interval(),
            telegram_token: None,
            plugin_path: default_plugin_path(),
            plugin_config: Default::default(),
        }
    }
}

fn default_port() -> u16 {
    80
}

fn default_tls_port() -> u16 {
    443
}

fn default_update_interval() -> u64 {
    1
}

fn default_database_path() -> String {
    let path = dirs::home_dir().unwrap().join(".tanoshi");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path.join("tanoshi.db").to_str().unwrap().to_string()
}

fn default_secret() -> String {
    let mut rng = thread_rng();
    let chars: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect();
    chars
}

fn default_plugin_path() -> String {
    let path = dirs::home_dir().unwrap().join(".tanoshi").join("plugins");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path.to_str().unwrap().to_string()
}
