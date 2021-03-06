use clap::{Arg, ArgMatches, SubCommand};
use clap;
use std::path::{Path, PathBuf};
use num_cpus;
use simulation::{ParSimInput, ParSimReport, Seed, SingSimInput};
use util::{load, save};
use errors::*;
use std::fs;
use std::process;
use std::io::Write;
use tokenizer::TokenStream;
use std::io::BufReader;
use std::ffi::OsStr;
use serde_json;
use util::{read_paths_in_dir, HenInfo};

mod util;
mod combine;
use app::util::{arg_application, arg_cleanup, arg_input, arg_output, arg_pegsfile, arg_report,
                GetMatch, SubCmd};
use app::combine::CombineConfig;

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
                .arg(arg_input())
                .arg(arg_cleanup())
                .arg(arg_output())
                .arg(arg_pegsfile())
                .arg(arg_application())
                .arg(
                    Arg::with_name("NTHREADS")
                        .long("nthreads")
                        .short("t")
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
                )
        )
        .subcommand(
            SubCommand::with_name("show")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Show content of simulation report.")
                .arg(arg_report())
                .arg(
                    Arg::with_name("WHAT")
                        .help("Show which aspects of the report")
                        .index(2)
                        .default_value("smart")
                        .case_insensitive(true)
                )
        )
        .subcommand(
            SubCommand::with_name("view")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Use egs_view to visualize simulation geometry of a report.")
                .arg(arg_report())
        )
        .subcommand(
            SubCommand::with_name("rerun")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Rerun a finished simulation.")
                .arg(arg_report())
                .arg(arg_output())
        )
        .subcommand(
            SubCommand::with_name("fmt")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Reformat .egsinp file.")
                .arg(
                    arg_input()
                        .help("Path to a .egsinp file that should be formatted.")
                )
        )
        .subcommand(
            SubCommand::with_name("split")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Split .egsinp file into chunks that are runnable on a cluster.")
                .arg(
                    arg_input()
                        .help("Path to a input file that should be split.")
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
                .arg(arg_output())
                .arg(arg_pegsfile())
                .arg(arg_application())
        )
        .subcommand(
            SubCommand::with_name("combine")
                .version(crate_version!())
                .author(crate_authors!())
                .about("Combine multiple .henout files into one.")
                .arg(arg_input())
                .arg(arg_output())
        )
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
            bail!("NTHREADS > 0 must hold.");
        }
        if self.nfiles == 0 {
            bail!("NFILES > 0 must hold.");
        }
        Ok(())
    }
}

impl SubCmd for SplitConfig {
    fn parse(m: &ArgMatches) -> Result<Self> {
        let outputpath = m.get_abspath("OUTPUT")?;
        let inputpath = m.get_abspath("INPUT")?;
        let nfiles = m.get_parse("NFILES")?;
        let nthreads = m.get_parse("NTHREADS")?;
        let application = m.get_string("APPLICATION")?;
        let pegsfile = m.get_string("PEGSFILE")?;
        let ret = SplitConfig {
            inputpath,
            outputpath,
            nthreads,
            nfiles,
            application,
            pegsfile,
        };
        ret.validate()?;
        Ok(ret)
    }

    fn run(&self) -> Result<()> {
        let prototype =
            SingSimInput::from_egsinp_path(&self.application, &self.inputpath, &self.pegsfile)?;
        let n = self.nthreads * self.nfiles;
        let ParSimInput {
            prototype,
            seeds,
            ncases,
        } = prototype.splitn(n)?;
        let chunksize = self.nthreads;
        let seeds = seeds.chunks(chunksize);
        let ncases = ncases.chunks(chunksize);
        let filestem = &self.inputpath
            .file_stem()
            .ok_or("Cannot get file_stem".to_string())?
            .to_str()
            .ok_or("to_str failed")?
            .to_string();
        for (i, (ncase, seed)) in ncases.zip(seeds).enumerate() {
            let filename = format!("{}_{}.heninp", filestem, i).to_string();
            let path = self.outputpath.join(filename);
            let psim = ParSimInput {
                prototype: prototype.clone(),
                ncases: ncase.to_vec(),
                seeds: seed.to_vec(),
            };
            save(&path, &psim)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct FormatConfig {
    input_path: PathBuf,
}

impl SubCmd for FormatConfig {
    fn parse(m: &ArgMatches) -> Result<Self> {
        let input_path = m.get_abspath("INPUT")?;
        Ok(Self { input_path })
    }

    fn run(&self) -> Result<()> {
        let formatted = {
            let file =
                fs::File::open(&self.input_path).chain_err(|| cannot_read(&self.input_path))?;
            let mut reader = BufReader::new(file);
            TokenStream::parse_reader(&mut reader)?.to_string()
        };
        fs::File::create(&self.input_path)
            .chain_err(|| cannot_create(&self.input_path))?
            .write_all(formatted.as_str().as_bytes())
            .chain_err(|| cannot_write(&self.input_path))
    }
}

#[derive(Debug)]
struct RerunConfig {
    path: PathBuf, // path to input
    outputpath: PathBuf,
}

impl SubCmd for RerunConfig {
    fn parse(m: &ArgMatches) -> Result<RerunConfig> {
        let path = m.get_abspath("PATH")?;
        let outputpath = m.get_abspath("OUTPUT")?;
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
    fn parse(m: &ArgMatches) -> Result<ViewConfig> {
        let path = m.get_abspath("PATH")?;
        Ok(ViewConfig { path })
    }

    fn run(&self) -> Result<()> {
        let ext = self.path
            .extension()
            .chain_err(|| format!("Cannot parse extension of {:?}", self.path))?
            .to_str()
            .chain_err(|| format!("Cannot convert ext to_str {:?}", self.path))?;
        match ext {
            "egsinp" => {
                let spath = self.path.to_str().unwrap();
                self.run_egsinp(spath)?;
            }
            "henout" => {
                self.run_par_sim_report()?;
            }
            _ => {
                bail!("Unknown extension {:?}", ext);
            }
        }
        Ok(())
    }
}

impl ViewConfig {
    fn run_par_sim_report(&self) -> Result<()> {
        let report: ParSimReport = load(&self.path)?;
        let content = report.input.prototype.content;
        let filestem = report.input.prototype.checksum;
        let filename = format!("{}.egsinp", filestem);
        let mut file = fs::File::create(&filename).chain_err(|| cannot_create(&filename))?;
        file.write_all(content.as_bytes())
            .chain_err(|| cannot_write(&filename))?;
        let out = self.run_egsinp(&filename);
        fs::remove_file(&filename).chain_err(|| cannot_remove(&filename))?;
        out?;
        Ok(())
    }

    fn run_egsinp(&self, filename: &str) -> Result<process::Output> {
        let ret = process::Command::new("egs_view")
            .args(&[filename])
            .output()
            .chain_err(|| "egs_view failed")?;
        if ret.status.success() {
            Ok(ret)
        } else {
            bail!("Bad exit status from egs_view: {:?}", ret);
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
    fn parse(m: &ArgMatches) -> Result<ShowConfig> {
        let path = m.get_abspath("PATH")?;
        // TODO get_enum
        let what = value_t!(m, "WHAT", ShowWhat).chain_err(|| "Could not parse argument")?;
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
    cleanup: bool,
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

    fn run_par_input(&self, p: &ParSimInput, output_path: &Path) -> Result<()> {
        match output_path.parent() {
            None => {}
            Some(d) => fs::create_dir_all(d)
                .chain_err(|| format!("Cannot create output directory at {:?}", output_path))?,
        };
        let out = p.run_with_cleanup_option(self.cleanup)
            .chain_err(|| "Error running parallel simulation")?
            .report();
        println!("{}", out);
        save(output_path, &out)
    }

    fn is_input_ext(s: &str) -> bool {
        (s == "egsinp") | (s == "heninp")
    }

    fn has_input_ext(path: &Path) -> bool {
        let ext = path.extension()
            .unwrap_or(OsStr::new("fail"))
            .to_str()
            .unwrap_or("fail");
        Self::is_input_ext(ext)
    }

    fn create_input_output_paths(&self) -> Result<Vec<(PathBuf, PathBuf)>> {
        let ret = if self.dir {
            read_paths_in_dir(&self.inputpath)?
                .iter()
                .filter(|p| Self::has_input_ext(p))
                .map(|inp| {
                    let filestem = inp.file_stem().unwrap();
                    let mut outp = self.outputpath.clone();
                    outp.push(filestem);
                    outp.set_extension("henout");
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
            let ext = inp.extension()
                .unwrap_or(OsStr::new("fail"))
                .to_str()
                .unwrap_or("fail");
            let sim = match ext {
                "heninp" => load(&inp)?,
                _ => self.create_sing_sim_input(&inp)?.split_fancy(
                    self.ncases.clone(),
                    self.seeds.clone(),
                    self.nthreads,
                )?,
            };
            self.run_par_input(&sim, &outp)?;
        }
        Ok(())
    }
}

impl SubCmd for RunConfig {
    fn parse(m: &ArgMatches) -> Result<RunConfig> {
        let inputpath = m.get_abspath("INPUT")?;
        let dir = inputpath.is_dir();
        let outputpath = m.get_abspath("OUTPUT")?;
        let application = m.get_string("APPLICATION")?;
        let pegsfile = m.get_string("PEGSFILE")?;
        let nthreads = m.get_parse("NTHREADS").unwrap_or(num_cpus::get());
        let seeds = match m.get("SEEDS") {
            Err(_) => None,
            Ok(s) => {
                let v: Vec<Seed> = serde_json::from_str(s).chain_err(|| "Cannot parse SEEDS")?;
                Some(v)
            }
        };
        let ncases = match m.get("NCASES") {
            Err(_) => None,
            Ok(s) => {
                let v: Vec<u64> = serde_json::from_str(s).chain_err(|| "Cannot parse NCASES")?;
                Some(v)
            }
        };
        let cleanup = m.get_parse("CLEANUP")?;
        let ret = RunConfig {
            inputpath,
            application,
            outputpath,
            pegsfile,
            nthreads,
            dir,
            ncases,
            seeds,
            cleanup,
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
        ("split", Some(m)) => SplitConfig::main(m),
        ("combine", Some(m)) => CombineConfig::main(m),
        ("", _) => Ok(println!(
            "Welcome to hen!\n{}\nTry hen --help",
            HenInfo::new()
        )),
        x => bail!("Unknown subcommand {:?}. Try hen --help", x),
    }
}
