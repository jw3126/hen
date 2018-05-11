use serde::Serialize;
use serde;
use serde_json;
use std::path::{Path, PathBuf};
use std;
use std::fs;
use std::fmt::Debug;

pub type Result<T> = std::result::Result<T, String>;

pub fn save<T: Serialize>(path: &Path, obj: &T) -> Result<()> {
    let file = fs::File::create(path).map_err(|err| format!("{:?}", err))?;
    serde_json::to_writer_pretty(file, obj).map_err(|err| format!("{:?}", err))?;
    return Ok(());
}

#[allow(dead_code)]
pub fn load<T>(path: &Path) -> Result<T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    let reader = fs::File::open(path).map_err(|e| format!("{:}", e).to_string())?;
    let ret: T = serde_json::from_reader(reader).map_err(|e| format!("{:?}", e).to_string())?;
    return Ok(ret);
}

pub fn debug_string<E: Debug>(e: E) -> String {
    format!("{:?}", e)
}

pub fn asset_path() -> PathBuf {
    let filepath = Path::new(file!())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_data")
        .canonicalize()
        .unwrap();
    return filepath;
}
