use serde::Serialize;
use std::collections::HashSet;
use std::hash::Hash;
use serde;
use serde_json as serde_format;
// use serde_yaml as serde_format // produces ugly yaml files
use std::path::{Path, PathBuf};
use std;
use std::fs;
use std::fmt::Debug;

pub type Result<T> = std::result::Result<T, String>;

pub fn save<T: Serialize>(path: &Path, obj: &T) -> Result<()> {
    let file = fs::File::create(path).map_err(|err| format!("{:?}", err))?;
    serde_format::to_writer_pretty(file, obj).map_err(|err| format!("{:?}", err))?;
    return Ok(());
}

#[allow(dead_code)]
pub fn load<T>(path: &Path) -> Result<T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    let reader = fs::File::open(path).map_err(|e| format!("{:}", e).to_string())?;
    let ret: T = serde_format::from_reader(reader).map_err(|e| format!("{:?}", e).to_string())?;
    return Ok(ret);
}

pub fn debug_string<E: Debug>(e: E) -> String {
    format!("{:?}", e)
}
#[allow(dead_code)]
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

pub fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}

#[test]
fn test_has_unique_elements() {
    assert!(!has_unique_elements(vec![10, 20, 30, 10, 50]));
    assert!(has_unique_elements(vec![10, 20, 30, 40, 50]));
    assert!(has_unique_elements(Vec::<u8>::new()));
    assert!(has_unique_elements(vec![(1,2),(1,3)]));
    assert!(!has_unique_elements(vec![(1,2),(1,2)]));
}
