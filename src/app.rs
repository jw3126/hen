use clap::{Arg, ArgMatches, SubCommand};
use clap;
use std::path::{Path, PathBuf};
use num_cpus;
use simulation::{ParSimReport, SingSimInput, Seed, ParSimInput};
use std::env::current_dir;
use util::{debug_string, load, save, Result};
use std::fs;
use std::process;
use std::io::Write;
use tokenizer::TokenStream;
use std::io::BufReader;
use std;
use std::ffi::OsStr;
use serde_json;

fn create_app() -> clap::App<'static, 'static> {
    clap::App::new("hen")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Run .egsinp files from anywhere.")
        .subcommand(
            SubCommand::with_name("run")
                .about("Run .egsinp files.")
                .version(crate_version!())
                .author(crate_authors!())
                .arg(
                    Arg::with_name("INPUT")
                        .index(1)
                        .takes_value(true)
                        .required(true)
                        .help("Name of the input file."),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .help("Name of the output file.")
                        .required(true), // TODO guess it
                )
                .arg(
                    Arg::with_name("PEGSFILE")
                        .short("p")
                        .long("pegsfile")
                        .help("Name of the pegsfile.")
                        .default_value("521icru")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("APPLICATION")
                        .short("a")
                        .long("app")
                        .help("Name of the application.")
                        .default_value("egs_chamber")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("NTHREADS")
                        .long("nthreads")
                        .help("Number of threads that should be used for the simulation. Defaults to the number of cores.")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("SEEDS")
                        .long("seeds")
                        .help("Random seeds that should be used. Format is e.g. [[1,2],[1,3],[4,5]].")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("NCASES")
                        .long("ncases")
                        .short("n")
                        .help("List of ncases that should be used. e.g [10000,10000,20000].")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("show")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Show content of simulation report.")
                .arg(
                    Arg::with_name("PATH")
                        .help("Path to a .json file containing simulation report.")
                        .index(1),
                )
                .arg(
                    Arg::with_name("WHAT")
                        .help("Show which aspects of the report")
                        .index(2)
                        .default_value("smart")
                        .case_insensitive(true)
                ),
        )
        .subcommand(
            SubCommand::with_name("view")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Use egs_view to visualize simulation geometry of a report.")
                .arg(
                    Arg::with_name("PATH")
                        .help("Path to a .json file containing simulation report.")
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("rerun")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Rerun a finished simulation.")
                .arg(
                    Arg::with_name("PATH")
                        .help("Path to a .json file containing simulation report.")
                        .index(1),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .help("Name of output file.")
                        .required(true), // guess it?
                ),
        )
        .subcommand(
            SubCommand::with_name("fmt")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Reformat .egsinp file.")
                .arg(
                    Arg::with_name("PATH")
                        .help("Path to a .egsinp file that should be formatted.")
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("split")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Split .egsinp file into chunks that are runnable on a cluster.")
                .arg(
                    Arg::with_name("INPUT")
                        .help("Path to a input file that should be split.")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("NFILES")
                        .long("nfiles")
                        .help("Number of files that an input file should be split into.")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("NTHREADS")
                        .long("nthreads")
                        .short("t")
                        .help("Number of threads that should be used for the simulation on each machine.")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .help("Path to directory, where output should be saved.")
                        .required(true),
                )
                .arg(
                    Arg::with_name("PEGSFILE")
                        .short("p")
                        .long("pegsfile")
                        .help("Name of the pegsfile.")
                        .default_value("521icru")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("APPLICATION")
                        .short("a")
                        .long("app")
                        .help("Name of the application.")
                        .default_value("egs_chamber")
                        .takes_value(true),
                ),
        )
}

trait SubCmd
where
    Self: std::marker::Sized,
{
    fn parse(m: &ArgMatches) -> Result<Self>;
    fn run(&self) -> Result<()>;
    fn main(m: &ArgMatches) -> Result<()> {
        Self::parse(m)?.run()
    }
}

#[derive(Debug)]
struct SplitConfig {
    outputpath: PathBuf,
    inputpath: PathBuf,
    nthreads: usize,
    nfiles: usize,
    application: String,
    pegsfile: String,
}

impl SplitConfig {
    fn validate(&self) -> Result<()> {
        if self.nthreads == 0 {
            return Err("NTHREADS > 0 must hold.".to_string())
        }
        if self.nfiles == 0 {
            return Err("NFILES > 0 must hold.".to_string())
        }
        Ok(())
    }
}

impl SubCmd for SplitConfig {
    fn parse(matches: &ArgMatches) -> Result<Self> {
        let outputpath = abspath_from_argmatches(matches, "OUTPUT")?;
        let inputpath = abspath_from_argmatches(matches, "INPUT")?;
        let nfiles = parse_from_argmatches(matches, "NFILES")?;
        let nthreads = parse_from_argmatches(matches, "NTHREADS")?;
        let application = matches.value_of("APPLICATION").unwrap().to_string();
        let pegsfile = matches.value_of("PEGSFILE").unwrap().to_string();
        let ret = SplitConfig { inputpath, outputpath, nthreads, nfiles, application, pegsfile};
        ret.validate()?;
        Ok(ret)
    }

    fn run(&self) -> Result<()> {
        let prototype = SingSimInput::from_egsinp_path(&self.application,
                                                       &self.inputpath, &self.pegsfile)?;
        let n = self.nthreads * self.nfiles;
        let ParSimInput {prototype, seeds, ncases} = prototype.splitn(n)?;
        let chunksize = self.nthreads;
        let seeds = seeds.chunks(chunksize);
        let ncases = ncases.chunks(chunksize);
        let filestem = &self.inputpath.file_stem()
            .ok_or("Cannot get file_stem".to_string())?
            .to_str().ok_or("to_str failed")?
            .to_string();
        let dir = &self.inputpath.parent()
            .ok_or("Cannot get parent of input_path".to_string())?;
        for (i, (ncase, seed)) in ncases.zip(seeds).enumerate() {
            let filename = format!("{}_{}.heninp",filestem,i).to_string();
            let path = dir.join(filename);
            let psim = ParSimInput {prototype:prototype.clone(),
                ncases:ncase.to_vec(),
                seeds:seed.to_vec()};
            save(&path, &psim)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct FormatConfig {
    path: PathBuf,
}

impl SubCmd for FormatConfig {
    fn parse(matches: &ArgMatches) -> Result<Self> {
        let path = abspath_from_argmatches(matches,"PATH")?;
        Ok(Self { path })
    }

    fn run(&self) -> Result<()> {
        let formatted = {
            let file = fs::File::open(&self.path).map_err(debug_string)?;
            let mut reader = BufReader::new(file);
            TokenStream::parse_reader(&mut reader)?.to_string()
        };
        fs::File::create(&self.path)
            .map_err(debug_string)?
            .write_all(formatted.as_str().as_bytes())
            .map_err(debug_string)
    }
}

#[derive(Debug)]
struct RerunConfig {
    path: PathBuf, // path to input
    outputpath: PathBuf,
}

impl SubCmd for RerunConfig {
    fn parse(matches: &ArgMatches) -> Result<RerunConfig> {
        let path = abspath_from_argmatches(matches, "PATH")?;
        let outputpath = abspath_from_argmatches(matches,"OUTPUT")?;
        Ok(RerunConfig { path, outputpath })
    }

    fn run(&self) -> Result<()> {
        let report: ParSimReport = load(&self.path)?;
        let sim = report.input;
        let out = sim.run()?.report();
        save(&self.outputpath, &out)?;
        return Ok(());
    }
}

#[derive(Debug)]
struct ViewConfig {
    path: PathBuf,
}

impl SubCmd for ViewConfig {
    fn parse(matches: &ArgMatches) -> Result<ViewConfig> {
        let path = abspath_from_argmatches(matches, "PATH")?;
        Ok(ViewConfig { path })
    }

    fn run(&self) -> Result<()> {
        let ext = self.path
            .extension()
            .ok_or(
                "Cannot parse extension of 
                   input path"
                    .to_string(),
            )?
            .to_str()
            .ok_or("Unicode problem with input path".to_string())?;
        match ext {
            "egsinp" => {
                let spath = self.path.to_str().unwrap();
                self.run_egsinp(spath)?;
            }
            "json" => {
                self.run_json()?;
            }
            _ => {
                return Err(format!("unknown extension {:?}", ext));
            }
        }
        Ok(())
    }
}

impl ViewConfig {
    fn run_json(&self) -> Result<()> {
        let report: ParSimReport = load(&self.path)?;
        let content = report.input.prototype.content;
        let filestem = report.input.prototype.checksum;
        let filename = format!("{}.egsinp", filestem);
        let mut file = fs::File::create(&filename).map_err(debug_string)?;
        file.write_all(content.as_bytes())
            .map_err(debug_string)?;
        let out = self.run_egsinp(&filename);
        fs::remove_file(filename).map_err(debug_string)?;
        out?;
        Ok(())
    }

    fn run_egsinp(&self, filename: &str) -> Result<process::Output> {
        let ret = process::Command::new("egs_view")
            .args(&[filename])
            .output()
            .map_err(|e| format!("egs_view failed: {:?}", e).to_string())?;
        if ret.status.success() {
            Ok(ret)
        } else {
            let msg = format!("{:?}", ret);
            Err(msg)
        }
    }
}

arg_enum!{
    #[derive(PartialEq, Debug)]
    pub enum ShowWhat {
        Output,
        Input,
        All,
        Smart
    }
}

#[derive(Debug)]
struct ShowConfig {
    path: PathBuf,
    what: ShowWhat,
}

impl SubCmd for ShowConfig {
    fn parse(matches: &ArgMatches) -> Result<ShowConfig> {
        let path = abspath_from_argmatches(matches, "PATH")?;
        let what = value_t!(matches, "WHAT", ShowWhat).map_err(debug_string)?;
        Ok(ShowConfig { path, what })
    }

    fn run(&self) -> Result<()> {
        let r: ParSimReport = load(&self.path)?;
        let s = match self.what {
            ShowWhat::Smart => r.to_string_smart(),
            ShowWhat::All => r.to_string_all(),
            ShowWhat::Input => r.to_string_input(),
            ShowWhat::Output => r.to_string_output(),
        };
        Ok(println!("{}", s))
    }
}

#[derive(Debug)]
struct RunConfig {
    inputpath: PathBuf,
    application: String,
    outputpath: PathBuf,
    pegsfile: String,
    seeds: Option<Vec<Seed>>,
    ncases: Option<Vec<u64>>,
    nthreads: usize,
    dir: bool, // run all files in a directory
}

impl RunConfig {
    pub fn validate(&self) -> Result<()> {
        if self.nthreads == 0 {
            return Err("NTHREADS > 0 must hold.".to_string())
        }
        if let Some(ref seeds) = self.seeds {
            if let Some(ref ncases) = self.ncases {
                if seeds.len() != ncases.len() {
                    return Err("SEEDS and NCASES must have the same length.".to_string());
                }
            }
        }
        Ok(())
    }

    fn create_sing_sim_input(&self, input_path: &Path) -> Result<SingSimInput> {
        SingSimInput::from_egsinp_path(&self.application, input_path, &self.pegsfile)
    }

    fn run_egsinp(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        match output_path.parent() {
            None => {}
            Some(d) => fs::create_dir_all(d).map_err(debug_string)?,
        };
        let out = self.create_sing_sim_input(input_path)?
            .split_fancy(self.ncases.clone(), self.seeds.clone(), self.nthreads)?
            .run()?
            .report();
        println!("{}", out);
        save(output_path, &out)
    }

    fn create_input_output_paths(&self) -> Result<Vec<(PathBuf, PathBuf)>> {
        let ret = if self.dir {
            fs::read_dir(self.inputpath.clone())
                .map_err(debug_string)?
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().unwrap_or(OsStr::new("fail")) == "egsinp")
                .map(|inp| {
                    let filestem = inp.file_stem().unwrap();
                    let mut outp = self.outputpath.clone();
                    outp.push(filestem);
                    outp.set_extension("json");
                    (inp.clone(), outp)
                })
                .collect()
        } else {
            vec![(self.inputpath.clone(), self.outputpath.clone())]
        };
        Ok(ret)
    }

    fn run(&self) -> Result<()> {
        let paths = self.create_input_output_paths()?;
        for (inp, outp) in paths {
            self.run_egsinp(&inp, &outp)?;
        }
        Ok(())
    }
}

fn parse_from_argmatches<T>(m:&ArgMatches, key:&str) -> Result<T> 
    where T: std::str::FromStr {

    let s =  m.value_of("key")
        .ok_or(format!("ArgMatches do not contain {}",key).to_string())?;
    let ret = s
        .parse::<T>()
        .map_err(|_| format!("Cannot parse {} from {}",key,s).to_string())?;
    Ok(ret)
}

fn abspath_from_argmatches(m:&ArgMatches, key: &str) -> Result<PathBuf> {
    let s = m.value_of(key)
        .ok_or(format!("ArgMatches do not contain {}", key).to_string())?;
    abspath_from_string(s)
}

fn abspath_from_string(s: &str) -> Result<PathBuf> {
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

impl SubCmd for RunConfig {
    fn parse(matches: &ArgMatches) -> Result<RunConfig> {
        let inputpath = abspath_from_argmatches(matches,"INPUT")?;
        let dir = inputpath.is_dir();
        let outputpath = abspath_from_argmatches(matches,"OUTPUT")?;
        let application = matches.value_of("APPLICATION")
            .unwrap().to_string();
        let pegsfile = matches.value_of("PEGSFILE")
            .unwrap().to_string();
        let nthreads = parse_from_argmatches(matches,"NTHREADS")
            .unwrap_or(num_cpus::get());
        let seeds = match matches.value_of("SEEDS") {
            None => None,
            Some(s) => {
                let v:Vec<Seed> = serde_json::from_str(s)
                    .map_err(|_|"Cannot parse SEEDS".to_string())?;
                Some(v)
            }
        };
        let ncases = match matches.value_of("NCASES") {
            None => None,
            Some(s) => {
                let v:Vec<u64> = serde_json::from_str(s)
                    .map_err(|_|"Cannot parse NCASES".to_string())?;
                Some(v)
            }
        };
        let ret = RunConfig {
            inputpath,
            application,
            outputpath,
            pegsfile,
            nthreads,
            dir,
            ncases,
            seeds,
        };
        ret.validate()?;
        Ok(ret)
    }

    fn run(&self) -> Result<()> {
        Self::run(self)
    }
}

pub fn app_main() -> Result<()> {
    let app = create_app();
    let matches = app.get_matches();
    match matches.subcommand() {
        ("run", Some(m)) => RunConfig::main(m),
        ("show", Some(m)) => ShowConfig::main(m),
        ("view", Some(m)) => ViewConfig::main(m),
        ("rerun", Some(m)) => RerunConfig::main(m),
        ("fmt", Some(m)) => FormatConfig::main(m),
        ("", _) => Err("Try hen --help".to_string()),
        x => Err(format!("Unknown subcommand {:?}. Try hen --help", x).to_string()),
    }
}
