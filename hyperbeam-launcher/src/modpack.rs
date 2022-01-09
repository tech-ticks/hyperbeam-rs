use hyperbeam_rtdx::modpack::{ModpackMetadata, MODPACK_BASE_PATH};
use hyperbeam_unity::texture_helpers;
use image::GenericImageView;
use pmdrtdx_bindings::Texture2D;
use semver::Version;
use serde::{Deserialize, Deserializer};
use std::error::Error;
use std::ffi::{CString, OsStr};
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::{fmt, fs};

#[derive(Debug)]
pub struct Modpack {
    pub metadata: ModpackMetadata,
    pub path: PathBuf,
    pub icon: Option<Result<NonNull<Texture2D>, Box<dyn Error>>>,
}

#[derive(Debug)]
pub struct InvalidModpack {
    pub error: Box<dyn Error>,
    pub path: PathBuf,
}

pub enum ModpackLoadResult {
    Success(Modpack),
    Invalid(InvalidModpack),
}

#[derive(Debug)]
struct MissingManifestError;

impl Error for MissingManifestError {}

impl fmt::Display for MissingManifestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "modpack.yaml missing in modpack root")
    }
}

#[derive(Debug)]
struct IDMismatchError;

impl Error for IDMismatchError {}

impl fmt::Display for IDMismatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Mismatch between folder name and modpack ID")
    }
}

#[derive(Debug)]
struct TargetError;

impl Error for TargetError {}

impl fmt::Display for TargetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Modpack target must be RTDX")
    }
}

#[derive(Debug)]
struct ImageDimensionsError {
    expected_width: i32,
    expected_height: i32,
}

impl Error for ImageDimensionsError {}

impl fmt::Display for ImageDimensionsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Wrong image dimensions, expected {}x{}",
            self.expected_width, self.expected_height
        )
    }
}

extern "C" {
    fn add_plugin(name: *const c_char) -> bool;
    fn load_plugin_modules() -> bool;
}

impl Modpack {
    fn new(path: &Path) -> Result<Modpack, Box<dyn Error>> {
        let mut metadata_path = path.to_owned();
        metadata_path.push(Path::new("modpack.yaml"));
        if !metadata_path.is_file() {
            return Err(Box::new(MissingManifestError {}));
        }

        let metadata_string = fs::read_to_string(&metadata_path)?;
        let metadata: ModpackMetadata = serde_yaml::from_str(&metadata_string)?;

        let folder_name = path.file_name().and_then(OsStr::to_str).unwrap();
        if folder_name != &metadata.id {
            return Err(Box::new(IDMismatchError {}));
        }

        if metadata.target != "RTDX" {
            return Err(Box::new(TargetError {}));
        }

        Ok(Modpack {
            metadata,
            path: path.to_owned(),
            icon: None,
        })
    }

    fn try_load_icon(&mut self) -> Result<NonNull<Texture2D>, Box<dyn Error>> {
        let mut path = self.path.clone();
        path.push("icon.png");
        // TODO: error if the image is not actually a PNG
        let image = image::io::Reader::open(path)?.decode()?.flipv();
        let rgba_image = image.to_rgba8();

        let expected_width = 250;
        let expected_height = 250;
        if image.width() as i32 != expected_width || image.height() as i32 != expected_height {
            return Err(Box::new(ImageDimensionsError {
                expected_width,
                expected_height,
            }));
        }

        Ok(texture_helpers::texture2d_from_bytes(
            rgba_image.as_raw(),
            expected_width,
            expected_height,
            false,
            false,
        ))
    }

    pub fn load_icon(&mut self) -> Result<NonNull<Texture2D>, &Box<dyn Error>> {
        if self.icon.is_none() {
            let result = self.try_load_icon();
            self.icon = Some(result);
        }
        return match self.icon.as_ref().unwrap() {
            Ok(icon) => Ok(*icon),
            Err(e) => Err(&e),
        };
    }

    pub fn unload_icon(&mut self) {
        if let Some(Ok(icon)) = self.icon.take() {
            unsafe {
                pmdrtdx_bindings::Object_1_Destroy_1(icon.as_ptr() as _, std::ptr::null_mut())
            }
        }
    }

    pub fn load_splash_image(&self) -> Result<NonNull<Texture2D>, Box<dyn Error>> {
        let mut path = self.path.clone();
        path.push("splash.png");
        let image = image::io::Reader::open(path)?.decode()?.flipv();
        let rgba_image = image.to_rgba8();

        let expected_width = 1280;
        let expected_height = 720;
        if image.width() as i32 != expected_width || image.height() as i32 != expected_height {
            return Err(Box::new(ImageDimensionsError {
                expected_width,
                expected_height,
            }));
        }

        Ok(texture_helpers::texture2d_from_bytes(
            rgba_image.as_raw(),
            expected_width,
            expected_height,
            false,
            false,
        ))
    }

    pub fn load_plugins(&self) {
        if let Ok(dir_contents) = fs::read_dir(self.path.join(Path::new("plugins"))) {
            dir_contents
                .filter_map(|f| f.ok())
                .map(|f| f.path())
                .filter(|f| f.is_file())
                .filter(|f| f.extension() == Some(OsStr::new("nro")))
                .for_each(|f| {
                    let plugin_path = CString::new(f.to_str().unwrap()).unwrap();
                    if !unsafe { add_plugin(plugin_path.as_ptr()) } {
                        panic!("[hyperbeam-launcher] Failed to add plugin.");
                    }
                });
        }

        if unsafe { load_plugin_modules() } {
            println!("[hyperbeam-launcher] Loaded plugin modules.");
        } else {
            panic!("Failed to load plugin modules!");
        }
    }
}

pub fn load_all_modpacks() -> Result<Vec<ModpackLoadResult>, Box<dyn Error>> {
    let mut modpacks = Vec::new();
    for dir in fs::read_dir(MODPACK_BASE_PATH)? {
        let dir = dir?;
        let mut path = dir.path();
        if path.is_dir() {
            match Modpack::new(&path) {
                Ok(modpack) => {
                    println!("Loaded modpack data: {:?}", modpack);
                    modpacks.push(ModpackLoadResult::Success(modpack));
                }
                Err(error) => {
                    let invalid_modpack = InvalidModpack { error, path };
                    eprintln!("Failed to load modpack data: {:?}", invalid_modpack);
                    modpacks.push(ModpackLoadResult::Invalid(invalid_modpack));
                }
            }
        }
    }

    Ok(modpacks)
}
