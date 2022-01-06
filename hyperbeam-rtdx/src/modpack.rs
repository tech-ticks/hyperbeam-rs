use semver::Version;
use serde::{Deserialize, Deserializer};

pub static MODPACK_BASE_PATH: &str = "sd:/atmosphere/contents/01003D200BAA2000/romfs/hyperbeam/modpacks";

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModpackMetadata {
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(deserialize_with = "from_semver")]
    pub version: Version,
    pub target: String,
}

fn from_semver<'de, D>(deserializer: D) -> Result<Version, D::Error>
    where
        D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Version::parse(&s).map_err(serde::de::Error::custom)
}
