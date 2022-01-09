use semver::Version;
use serde::{Deserialize, Deserializer};

pub fn from_semver<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Version::parse(&s).map_err(serde::de::Error::custom)
}
