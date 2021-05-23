use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::BTreeMap;
use std::iter;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_secret")]
    pub secret: String,
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
    #[serde(default = "default_plugin_path")]
    pub plugin_path: String,
    #[serde(default = "BTreeMap::new")]
    pub plugin_config: BTreeMap<String, serde_yaml::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: default_port(),
            database_path: default_database_path(),
            secret: default_secret(),
            update_interval: default_update_interval(),
            plugin_path: default_plugin_path(),
            plugin_config: Default::default(),
        }
    }
}

fn default_port() -> u16 {
    80
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
    let chars = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect();
    String::from_utf8(chars).unwrap()
}

fn default_plugin_path() -> String {
    let path = dirs::home_dir().unwrap().join(".tanoshi").join("plugins");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path.to_str().unwrap().to_string()
}

impl Config {
    pub fn open<P: AsRef<std::path::Path>>(path: Option<P>) -> Result<Config, anyhow::Error> {
        match path {
            Some(path) => {
                let file = std::fs::File::open(path)?;
                serde_yaml::from_reader(file).map_err(Self::map_yaml_error)
            }
            None => {
                let config = Self::default();
                let path = dirs::home_dir().unwrap().join(".tanoshi");
                let _ = std::fs::create_dir_all(path.clone());
                let path = path.join("config.yml");
                let _ = std::fs::write(&path, serde_yaml::to_string(&config).unwrap());
                info!("Write default config at {:?}", path);
                Ok(config)
            }
        }
    }

    fn map_yaml_error(err: serde_yaml::Error) -> anyhow::Error {
        anyhow::anyhow!("error parse yaml: {}", err)
    }
}
