use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{collections::BTreeMap, iter, path::PathBuf};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_path: String,
    #[serde(default = "default_secret")]
    pub secret: String,
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
    pub plugin_path: String,
    #[serde(default = "BTreeMap::new")]
    pub plugin_config: BTreeMap<String, serde_yaml::Value>,
}

impl Config {
    fn new(database_path: String, plugin_path: String) -> Self {
        Self {
            port: default_port(),
            database_path,
            secret: default_secret(),
            update_interval: default_update_interval(),
            plugin_path,
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

fn default_secret() -> String {
    let mut rng = thread_rng();
    let chars = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect();
    String::from_utf8(chars).unwrap()
}

impl Config {
    pub fn open(path: Option<String>) -> Result<Config, Box<dyn std::error::Error>> {
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
                serde_yaml::from_reader(file)
                    .map_err(|err| format!("error parse yaml: {}", err).into())
            }
            Err(_) => {
                let parent = config_path.parent().unwrap();
                let database_path = parent
                    .join("tanoshi")
                    .with_extension("db")
                    .to_str()
                    .unwrap()
                    .to_string();
                let plugin_path = parent.join("plugins").to_str().unwrap().to_string();
                let config = Config::new(database_path, plugin_path);

                let _ = std::fs::create_dir_all(config_path.parent().expect("should have parent"));
                let _ = std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap());
                info!("Write default config at {:?}", config_path);
                Ok(config)
            }
        }
    }
}
