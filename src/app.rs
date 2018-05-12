use clap::{Arg, ArgMatches, SubCommand};
use clap;
use std::path::{Path, PathBuf};
use num_cpus;
use simulation::{ParallelSimulation, ParallelSimulationReport, SingleSimulation};
use std::env::current_dir;
use util::{debug_string, load, save, Result};
use std::fs;
use std::process;
use std::io::Write;
use tokenizer::TokenStream;
use std::io::BufReader;
use std;
static DEFAULT_APP: &str = "egs_chamber";
static DEFAULT_PEGSFILE: &str = "521icru";

fn create_app() -> clap::App<'static, 'static> {
    clap::App::new("egscli")
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
                        .help("Set the input file."),
                )
                .arg(
                    Arg::with_name("FORCE")
                        .short("f")
                        .long("force")
                        .help("Overwrite existing input/output files."),
                )
                .arg(
                    Arg::with_name("PEGSFILE")
                        .short("p")
                        .long("pegsfile")
                        .help("Name of the pegsfile.")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("OUTPUT")
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .help("Name of output file.")
                        .required(true), // TODO guess it
                )
                .arg(
                    Arg::with_name("APPLICATION")
                        .short("a")
                        .long("app")
                        .help("Name of the application. Default: egs_chamber.")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("NTHREADS")
                        .long("nthreads")
                        .help("Number of threads that should be used for the simulation.")
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
        let spath = matches.value_of("PATH").unwrap();
        let path = abspath_from_string(spath)?;
        let soutputpath = matches.value_of("OUTPUT").unwrap();
        let outputpath = abspath_from_string(soutputpath)?;
        Ok(RerunConfig { path, outputpath })
    }

    fn run(&self) -> Result<()> {
        let report: ParallelSimulationReport = load(&self.path)?;
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
                return Err(format!("unknown extension {:?}", ext));
            }
        }
        Ok(())
    }
}

impl ViewConfig {
    fn run_json(&self) -> Result<()> {
        let report: ParallelSimulationReport = load(&self.path)?;
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
            .map_err(|e| format!("egs_view failed: {:?}", e).to_string())?;
        if ret.status.success() {
            Ok(ret)
        } else {
            let msg = format!("{:?}", ret);
            Err(msg)
        }
    }
}

#[derive(Debug)]
struct ShowConfig {
    path: PathBuf,
}

impl SubCmd for ShowConfig {
    fn parse(matches: &ArgMatches) -> Result<ShowConfig> {
        let spath = matches.value_of("PATH").unwrap();
        let path = abspath_from_string(spath)?;
        Ok(ShowConfig { path })
    }

    fn run(&self) -> Result<()> {
        let report: ParallelSimulationReport = load(&self.path)?;
        Ok(println!("{}", report))
    }
}

#[derive(Debug)]
struct RunConfig {
    inputpath: PathBuf,
    application: String,
    outputpath: PathBuf,
    pegsfile: String,
    force: bool,
    nthreads: usize,
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
            Err("NTHREADS > 0 must hold.".to_string())
        } else {
            Ok(())
        }
    }

    fn create_single_simulation(&self) -> Result<SingleSimulation> {
        SingleSimulation::from_egsinp_path(&self.application, &self.inputpath, &self.pegsfile)
    }

    fn create_parallel_simulation(&self) -> Result<ParallelSimulation> {
        let prototype = self.create_single_simulation()?;
        return Ok(prototype.splitn(self.nthreads));
    }
}

impl SubCmd for RunConfig {
    fn parse(matches: &ArgMatches) -> Result<RunConfig> {
        let force = matches.is_present("FORCE");
        let sinputpath = matches.value_of("INPUT").unwrap();
        let inputpath = abspath_from_string(sinputpath)?;
        let soutputpath = matches.value_of("OUTPUT").unwrap();
        let outputpath = abspath_from_string(soutputpath)?;
        let application = matches
            .value_of("APPLICATION")
            .unwrap_or(DEFAULT_APP)
            .to_string();
        let pegsfile = matches
            .value_of("PEGSFILE")
            .unwrap_or(DEFAULT_PEGSFILE)
            .to_string();
        let nthreads = match matches.value_of("NTHREADS") {
            None => num_cpus::get(),
            Some(snthreads) => snthreads
                .parse::<usize>()
                .map_err(|_| "Cannot parse NTHREADS".to_string())?,
        };
        let ret = RunConfig {
            inputpath,
            application,
            outputpath,
            pegsfile,
            force,
            nthreads,
        };
        ret.validate()?;
        Ok(ret)
    }

    fn run(&self) -> Result<()> {
        let sim = self.create_parallel_simulation()?;
        let out = sim.run()?.report();
        save(&self.outputpath, &out)?;
        return Ok(());
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
        x => Err(format!("Unknown subcommand {:?}. Try hen --help", x).to_string()),
    }
}
