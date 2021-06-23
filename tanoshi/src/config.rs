use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{collections::BTreeMap, iter, path::PathBuf};

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
    pub fn open(path: Option<String>) -> Result<Config, anyhow::Error> {
        let config_path = match path {
            Some(p) => PathBuf::from(p.clone()),
            None => dirs::home_dir()
                .expect("should be home dir, or define path")
                .join(".tanoshi")
                .join("config.yml"),
        };

        match std::fs::File::open(config_path.clone()) {
            Ok(file) => {
                info!("Open config from {:?}", config_path);
                serde_yaml::from_reader(file).map_err(Self::map_yaml_error)
            }
            Err(_) => {
                let config = Self::default();
                let _ = std::fs::create_dir_all(config_path.parent().expect("should have parent"));
                let _ = std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap());
                info!("Write default config at {:?}", config_path);
                Ok(config)
            }
        }
    }

    fn map_yaml_error(err: serde_yaml::Error) -> anyhow::Error {
        anyhow::anyhow!("error parse yaml: {}", err)
    }
}
