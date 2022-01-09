use lazy_static;
use serde::Deserialize;
use std::error::Error;
use std::fs;

pub static CONFIG_PATH: &str =
    "sd:/atmosphere/contents/01003D200BAA2000/romfs/hyperbeam/config.yaml";

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub auto_launch: Option<String>,
}

lazy_static::lazy_static! {
    static ref CONFIG: Config = read_config().unwrap_or_else(|err| {
        eprintln!("Failed to read config: {}", err);
        Config::default()
    });
}

pub fn get_config() -> &'static Config {
    &*CONFIG
}

fn read_config() -> Result<Config, Box<dyn Error>> {
    let config_string = fs::read_to_string(CONFIG_PATH)?;
    let config: Config = serde_yaml::from_str(&config_string)?;
    Ok(config)
}
