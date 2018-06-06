use serde::Serialize;
use std::collections::HashSet;
use std::hash::Hash;
use serde;
use serde_json as serde_format;
// use serde_yaml as serde_format // produces ugly yaml files
use std::path::{Path, PathBuf};
use std::fs;
use std::fmt;
use errors::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HenInfo {
    pub version: String,
    pub commit: String,
    pub timestamp: String,
    pub api_version: usize,
}

impl HenInfo {
    pub fn new() -> Self {
        let commit = env!("HEN_COMMIT_HASH").to_string();
        let timestamp = env!("HEN_COMMIT_TIME").to_string();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let api_version = 0;
        Self {
            version,
            commit,
            timestamp,
            api_version,
        }
    }
}

impl fmt::Display for HenInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "commit version: {}", self.version)?;
        writeln!(f, "commit hash: {}", self.commit)?;
        writeln!(f, "commit time: {}", self.timestamp)?;
        writeln!(f, "api version: {}", self.api_version)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithMeta<T> {
    hen: HenInfo,
    #[serde(flatten)]
    content: T,
}

impl<T> WithMeta<T> {
    pub fn new(content: T) -> Self {
        let hen = HenInfo::new();
        WithMeta { content, hen }
    }
}

pub fn save<T: Serialize>(path: &Path, obj: &T) -> Result<()> {
    let file = fs::File::create(path).chain_err(|| cannot_create(&path))?;
    let obj = WithMeta::new(obj);
    serde_format::to_writer_pretty(file, &obj).chain_err(|| cannot_write(&path))?;
    Ok(())
}

#[allow(dead_code)]
pub fn load<T>(path: &Path) -> Result<T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    let reader = fs::File::open(path).chain_err(|| cannot_read(&path))?;
    let ret: WithMeta<T> = serde_format::from_reader(reader).chain_err(|| cannot_read(&path))?;
    Ok(ret.content)
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

pub fn read_paths_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let ret = fs::read_dir(dir)
        .chain_err(|| cannot_read(&dir))?
        .map(|entry| entry.unwrap().path())
        .collect();
    Ok(ret)
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
    assert!(has_unique_elements(vec![(1, 2), (1, 3)]));
    assert!(!has_unique_elements(vec![(1, 2), (1, 2)]));
    assert!(!has_unique_elements(vec!["a".to_string(), "a".to_string()]));
}
