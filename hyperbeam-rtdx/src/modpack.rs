use crate::serialization;
use semver::Version;
use serde::Deserialize;

pub static MODPACK_BASE_PATH: &str =
    "sd:/atmosphere/contents/01003D200BAA2000/romfs/hyperbeam/modpacks";

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModpackMetadata {
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(deserialize_with = "serialization::from_semver")]
    pub version: Version,
    pub target: String,
}
