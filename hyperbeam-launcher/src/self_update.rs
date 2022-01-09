use hyperbeam_rtdx::serialization;
use semver::Version;
use serde::Deserialize;
use std::io::{self, Cursor};
use std::fs::{self, File};
use std::error;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use zip::ZipArchive;

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/tech-ticks/hyperbeam-rs/releases/latest";
const UPDATE_BASE_PATH: &str = "sd:/atmosphere/contents/01003D200BAA2000/romfs";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize, Debug)]
struct GitHubReleaseAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    name: String,
    #[serde(deserialize_with = "serialization::from_semver")]
    tag_name: Version,
    body: String,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Update {
    pub version: Version,
    pub download: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UpdateCheckResult {
    NoUpdate,
    UpdateAvailable(Update),
}

pub struct ProgressPercentage(f32);

pub enum UpdateProgress {
    Downloading(ProgressPercentage),
    Installing,
    Finished,
    Err(Error),
}

#[derive(Debug)]
struct NoReleaseAssetError;

impl error::Error for NoReleaseAssetError {}

pub type UpdateCheckReceiver = Receiver<Result<UpdateCheckResult, Error>>;
pub type UpdateReceiver = Receiver<UpdateProgress>;
type Error = Box<dyn error::Error + Send + Sync>;

impl Display for NoReleaseAssetError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "No release asset found")
    }
}

fn get_latest_release() -> Result<GitHubRelease, Error> {
    Ok(minreq::get(LATEST_RELEASE_URL)
        .with_header("User-Agent", "hyperbeam-launcher")
        .with_timeout(10)
        .send()?
        .json()?)
}

pub fn start_check_self_update() -> UpdateCheckReceiver {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let message: Result<UpdateCheckResult, Error> = match get_latest_release() {
            Ok(res) => {
                let update_available = true; // TODO: restore !!!!!!!!!!!!!!!!!!             res.tag_name > Version::parse(VERSION).unwrap();
                if update_available {
                    println!("{:?}", res.assets);
                    let launcher_asset = res.assets.iter().find(|item| {
                        item.name.starts_with("libhyperbeam_launcher")
                            && item.name.ends_with(".nro")
                    });
                    match launcher_asset {
                        Some(asset) => Ok(UpdateCheckResult::UpdateAvailable(Update {
                            version: res.tag_name,
                            download: asset.browser_download_url.clone(),
                        })),
                        None => Err(Into::<Error>::into(NoReleaseAssetError)),
                    }
                } else {
                    Ok(UpdateCheckResult::NoUpdate)
                }
            }
            Err(err) => Err(Into::<Error>::into(err)),
        };
        if let Err(error) = tx.send(message) {
            eprintln!("Update check thread failed to send message: {:?}", error);
        }
    });

    rx
}

impl Update {
    pub fn start_update(self) -> UpdateReceiver {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            match self
                .download_update(&tx)
                .and_then(|bytes| self.install_update(bytes, &tx))
            {
                Ok(_) => tx.send(UpdateProgress::Finished),
                Err(error) => tx.send(UpdateProgress::Err(error))
            };
        });

        rx
    }

    fn download_update(&self, sender: &Sender<UpdateProgress>) -> Result<Vec<u8>, Error> {
        let response = minreq::get(self.download.as_str()).send_lazy()?;

        let mut bytes = Vec::new();
        let mut progress_update_counter: u64 = 0;
        for result in response {
            let (byte, length) = result?;
            bytes.reserve(length);
            bytes.push(byte);
            progress_update_counter += 1;

            if progress_update_counter == 1024 {
                sender.send(UpdateProgress::Downloading(ProgressPercentage(
                    bytes.len() as f32 / bytes.capacity() as f32,
                )));
                progress_update_counter = 0;
            }
        }

        Ok(bytes)
    }

    fn install_update(&self, bytes: Vec<u8>, sender: &Sender<UpdateProgress>) -> Result<(), Error> {
        let mut cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let path = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let out_path = PathBuf::from(UPDATE_BASE_PATH).join(path);
            if (&*file.name()).ends_with('/') {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(p) = out_path.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = File::create(&out_path)?;
                io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok(())
    }
}
