use clap::ArgMatches;
use std::path::{Path,PathBuf};
use std::env::current_dir;
use std;
use util::Result;

pub trait SubCmd
where
    Self: std::marker::Sized,
{
    fn parse(m: &ArgMatches) -> Result<Self>;
    fn run(&self) -> Result<()>;
    fn main(m: &ArgMatches) -> Result<()> {
        Self::parse(m)?.run()
    }
}

pub trait GetMatch {

    fn get(&self, key:&str) -> Result<&str>;
    fn get_string(&self, key:&str) -> Result<String> {
        let ret = self.get(key)?.to_string();
        Ok(ret)
    }

    fn get_abspath(&self, key:&str) -> Result<PathBuf> {
        let s = self.get(key)?;
        abspath_from_string(s)
    }

    fn get_parse<T>(&self, key:&str) -> Result<T> 
        where T: std::str::FromStr {

        let s = self.get(key)?;
        let ret = s
            .parse::<T>()
            .map_err(|_| format!("Cannot parse {} from {}",key,s).to_string())?;
        Ok(ret)
    }

}

impl<'t> GetMatch for ArgMatches<'t>
{
    fn get(&self, key:&str) -> Result<&str> {
        let s =  self
            .value_of(key)
            .ok_or(format!("ArgMatches do not contain {}",key));
        s
    }
}

pub fn abspath_from_string(s: &str) -> Result<PathBuf> {
    let isabs = Path::new(s).is_absolute();
    let mut path = PathBuf::new();
    if isabs {
        path.push(s);
    } else {
        let dir = current_dir().map_err(|e| format!("{:?}", e))?;
        path.push(dir);
        path.push(s);
    }
    Ok(path)
}

