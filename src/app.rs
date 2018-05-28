use clap::{Arg, ArgMatches, SubCommand};
use clap;
use std::path::{Path, PathBuf};
use num_cpus;
use simulation::{ParSimReport, SingSimInput, Seed};
use std::env::current_dir;
use util::{debug_string, load, save};
use errors::*;
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
                    Arg::with_name("PEGSFILE")
                        .short("p")
                        .long("pegsfile")
                        .help("Name of the pegsfile.")
                        .default_value("521icru")
                        .takes_value(true),
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
struct FormatConfig {
    path: PathBuf,
}

impl SubCmd for FormatConfig {
    fn parse(matches: &ArgMatches) -> Result<Self> {
        let spath = matches.value_of("PATH").unwrap();
        let path = abspath_from_string(spath)?;
        Ok(Self { path })
    }

    fn run(&self) -> Result<()> {
        let formatted = {
            let file = fs::File::open(&self.path)
                .chain_err(||"Cannot read")?;
            let mut reader = BufReader::new(file);
            TokenStream::parse_reader(&mut reader)?.to_string()
        };
        fs::File::create(&self.path)
            .map_err(debug_string)?
            .write_all(formatted.as_str().as_bytes())
            .chain_err(||"Cannot write to file")
    }
}

#[derive(Debug)]
struct RerunConfig {
    path: PathBuf, // path to input
    outputpath: PathBuf,
}

impl SubCmd for RerunConfig {
    fn parse(matches: &ArgMatches) -> Result<RerunConfig> {
        let spath = matches.value_of("PATH").unwrap();
        let path = abspath_from_string(spath)?;
        let soutputpath = matches.value_of("OUTPUT").unwrap();
        let outputpath = abspath_from_string(soutputpath)?;
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
        let spath = matches.value_of("PATH").unwrap();
        let path = abspath_from_string(spath)?;
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
                bail!("unknown extension {:?}", ext);
            }
        }
        Ok(())
    }
}

impl ViewConfig {
    fn run_json(&self) -> Result<()> {
        let report: ParSimReport = load(&self.path)?;
        let input_content = report.input.prototype.input_content;
        let filestem = report.input.prototype.checksum;
        let filename = format!("{}.egsinp", filestem);
        let mut file = fs::File::create(&filename).map_err(debug_string)?;
        file.write_all(input_content.as_bytes())
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
            .chain_err(||"egs_view failed")?;
        if ret.status.success() {
            Ok(ret)
        } else {
            bail!("{:?}", ret);
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
        let spath = matches.value_of("PATH").unwrap();
        let path = abspath_from_string(spath)?;
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

impl RunConfig {
    pub fn validate(&self) -> Result<()> {
        if self.nthreads == 0 {
            bail!("NTHREADS > 0 must hold.");
        }
        if let Some(ref seeds) = self.seeds {
            if let Some(ref ncases) = self.ncases {
                if seeds.len() != ncases.len() {
                    bail!("SEEDS and NCASES must have the same length.");
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

impl SubCmd for RunConfig {
    fn parse(matches: &ArgMatches) -> Result<RunConfig> {
        let sinputpath = matches.value_of("INPUT").unwrap();
        let inputpath = abspath_from_string(sinputpath)?;
        let dir = inputpath.is_dir();
        let soutputpath = matches.value_of("OUTPUT").unwrap();
        let outputpath = abspath_from_string(soutputpath)?;
        let application = matches.value_of("APPLICATION").unwrap().to_string();
        let pegsfile = matches.value_of("PEGSFILE").unwrap().to_string();
        let nthreads = match matches.value_of("NTHREADS") {
            None => num_cpus::get(),
            Some(snthreads) => snthreads
                .parse::<usize>()
                .map_err(|_| "Cannot parse NTHREADS".to_string())?,
        };
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
        ("", _) => bail!("Try hen --help"),
        x => bail!("Unknown subcommand {:?}. Try hen --help", x)
    }
}
