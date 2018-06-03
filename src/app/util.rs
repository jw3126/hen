use clap::{Arg, ArgMatches};
use std::path::{Path, PathBuf};
use std::env::current_dir;
use std;
use util::Result;

pub fn arg_input() -> Arg<'static, 'static> {
    Arg::with_name("INPUT")
        .index(1)
        .takes_value(true)
        .required(true)
        .help("Name of the input file.")
}

pub fn arg_output() -> Arg<'static, 'static> {
    Arg::with_name("OUTPUT")
        .short("o")
        .long("output")
        .takes_value(true)
        .help("Path where output should be stored.")
        .required(true)
}

pub fn arg_pegsfile() -> Arg<'static, 'static> {
    Arg::with_name("PEGSFILE")
        .short("p")
        .long("pegsfile")
        .help("Name of the pegsfile.")
        .default_value("521icru")
        .takes_value(true)
}

pub fn arg_application() -> Arg<'static, 'static> {
    Arg::with_name("APPLICATION")
        .short("a")
        .long("app")
        .help("Name of the application.")
        .default_value("egs_chamber")
        .takes_value(true)
}

pub fn arg_report() -> Arg<'static, 'static> {
    Arg::with_name("PATH")
        .help("Path to a .henout file containing simulation report.")
        .index(1)
}

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
    fn get(&self, key: &str) -> Result<&str>;
    fn get_string(&self, key: &str) -> Result<String> {
        let ret = self.get(key)?.to_string();
        Ok(ret)
    }

    fn get_abspath(&self, key: &str) -> Result<PathBuf> {
        let s = self.get(key)?;
        abspath_from_string(s)
    }

    fn get_parse<T>(&self, key: &str) -> Result<T>
    where
        T: std::str::FromStr,
    {
        let s = self.get(key)?;
        let ret = s.parse::<T>()
            .map_err(|_| format!("Cannot parse {} from {}", key, s).to_string())?;
        Ok(ret)
    }
}

impl<'t> GetMatch for ArgMatches<'t> {
    fn get(&self, key: &str) -> Result<&str> {
        let s = self.value_of(key)
            .ok_or(format!("ArgMatches do not contain {}", key));
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
