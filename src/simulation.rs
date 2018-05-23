use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufReader;
use std::io::{Read, Write};
use std::process::{Command, Output};
use std::fs;
use rayon::prelude::*;
use tokenizer::TokenStream;
use sha3;
use sha3::Digest;
use std;
use uncertain::UncertainF64;
use output_parser;
use std::fmt;
use util::Result;
use std::result::Result as StdResult;

pub type Seed = (usize, usize); // is this correct integer type?

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingSimInput {
    pub application: String,
    pub input_content: String,
    pub pegsfile: String,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParSimInput {
    pub prototype: SingSimInput,
    pub seeds: Vec<Seed>,
    pub ncases: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingSimFinished {
    pub input: SingSimInput,
    pub stderr: String,
    pub stdout: String,
    pub exit_status: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Omitable<T> {
    Omitted,
    Fail(String),
    Available(T),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingSimParsedOutput {
    pub dose: Result<Vec<(String, UncertainF64)>>,
    pub total_cpu_time: Result<f64>,
    pub simulation_finished: Result<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingSimReport {
    pub input: SingSimInput,
    pub stderr: Omitable<String>,
    pub stdout: Omitable<String>,
    pub exit_status: Omitable<i32>,
    pub dose: Omitable<Vec<(String, UncertainF64)>>,
    pub total_cpu_time: Omitable<f64>,
    pub simulation_finished: Omitable<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParSimFinished {
    pub input: ParSimInput,
    pub outputs: Vec<SingSimFinished>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParSimReport {
    pub input: ParSimInput,
    pub outputs: Omitable<Vec<SingSimReport>>,

    pub total_cpu_time: Omitable<f64>,
    pub simulation_finished: Omitable<bool>,
    pub dose: Omitable<Vec<(String, UncertainF64)>>,
}

impl ParSimInput {
    pub fn run(&self) -> Result<ParSimFinished> {
        let stream = TokenStream::parse_string(&(self.prototype.input_content))?;
        let streams = stream.split(&self.seeds, &self.ncases)?;
        let application = &self.prototype.application;
        let pegsfile = &self.prototype.pegsfile;
        let create_sim = |content: String| {
            SingSimInput::new(application.clone(), content.clone(), pegsfile.clone())
        };
        let results: Vec<SingSimFinished> = streams
            .par_iter()
            .map(|s| s.to_string())
            .map(create_sim)
            .map(|sim| sim.clone().run())
            .collect();
        let ret = ParSimFinished {
            input: self.clone(),
            outputs: results,
        };
        return Ok(ret);
    }
}

fn egs_home_path() -> PathBuf {
    let mut path = PathBuf::new();
    let segs_home = std::env::var("EGS_HOME").expect("Cannot find EGS_HOME");
    path.push(segs_home);
    path
}

impl SingSimInput {
    pub fn new(application: String, input_content: String, pegsfile: String) -> Self {
        let digest = sha3::Sha3_256::digest(input_content.as_bytes());
        let checksum = format!("{:x}", digest);
        let sim = SingSimInput {
            application,
            input_content,
            pegsfile,
            checksum,
        };
        return sim;
    }

    pub fn from_egsinp_path(application: &str, path: &Path, pegsfile: &str) -> Result<Self> {
        let mut file = File::open(path).map_err(|err| format!("{:?}", err))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|err| format!("{:?}", err))?;
        return Ok(Self::new(
            application.to_string(),
            content,
            pegsfile.to_string(),
        ));
    }

    fn run_cmd(&self) -> std::io::Result<Output> {
        let mut file = fs::File::create(self.path_exec_with_ext("egsinp"))?;
        file.write_all(self.input_content.as_bytes()).unwrap();

        let ret = Command::new(self.application.clone())
            .args(&["-i", self.checksum.as_str(), "-p", self.pegsfile.as_str()])
            .output();

        return ret;
    }

    pub fn run(&self) -> SingSimFinished {
        let out = self.run_cmd().unwrap();
        let ret = SingSimFinished {
            input: self.clone(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            exit_status: out.status.code().unwrap_or(-1),
        };
        self.cleanup();
        ret
    }

    pub fn cleanup(&self) -> () {
        for ext in ["egsinp", "egsdat", "ptracks"].iter() {
            let path = self.path_exec_with_ext(&ext);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn app_dir(&self) -> PathBuf {
        let mut path = egs_home_path();
        path.push(self.application.clone());
        path
    }

    fn path_exec_with_ext(&self, ext: &str) -> PathBuf {
        let mut path = self.app_dir();
        path.push(&self.checksum);
        assert!(path.set_extension(ext));
        path
    }

    pub fn split(self, seeds: Vec<Seed>, ncases: Vec<u64>) -> ParSimInput {
        let prototype = self;
        ParSimInput {
            seeds,
            prototype,
            ncases,
        }
    }

    pub fn splitn(self, n: usize) -> Result<ParSimInput> {
        let stream = TokenStream::parse_string(&(self.input_content))?;
        let seeds = stream.generate_seeds(n)?;
        let ncases = stream.generate_ncases(n)?;
        let ret = self.split(seeds, ncases);
        Ok(ret)
    }
}

impl<T> Omitable<T> {
    pub fn from_result(r: Result<T>) -> Omitable<T> {
        match r {
            Ok(value) => Omitable::Available(value),
            Err(s) => Omitable::Fail(s),
        }
    }

    pub fn into_result(self) -> Result<T> {
        match self {
            Omitable::Fail(s) => Err(s),
            Omitable::Omitted => Err("Omitted".to_string()),
            Omitable::Available(t) => Ok(t),
        }
    }

    #[allow(dead_code)]
    pub fn map<U, F: Fn(T) -> U>(self, f: F) -> Omitable<U> {
        match self {
            Omitable::Available(value) => Omitable::Available(f(value)),
            Omitable::Fail(s) => Omitable::Fail(s.clone()),
            Omitable::Omitted => Omitable::Omitted,
        }
    }

    pub fn map2<S, U, F: Fn(S, T) -> U>(f: F, s: Omitable<S>, t: Omitable<T>) -> Omitable<U> {
        match s {
            Omitable::Fail(msg) => Omitable::Fail(msg),

            Omitable::Omitted => match t {
                Omitable::Fail(msg) => Omitable::Fail(msg),
                _ => Omitable::Omitted,
            },

            Omitable::Available(s_val) => match t {
                Omitable::Available(t_val) => Omitable::Available(f(s_val, t_val)),
                Omitable::Omitted => Omitable::Omitted,
                Omitable::Fail(msg) => Omitable::Fail(msg),
            },
        }
    }
}

impl SingSimFinished {
    fn parse_output(&self) -> SingSimParsedOutput {
        let mut reader = BufReader::new(self.stdout.as_bytes());
        return output_parser::parse_simulation_output(&mut reader);
    }

    pub fn report(&self) -> SingSimReport {
        let out = self.parse_output();
        SingSimReport {
            input: self.input.clone(),
            stderr: Omitable::Available(self.stderr.clone()),
            stdout: Omitable::Available(self.stdout.clone()),
            exit_status: Omitable::Available(self.exit_status),
            dose: Omitable::from_result(out.dose),
            total_cpu_time: Omitable::from_result(out.total_cpu_time),
            simulation_finished: Omitable::from_result(out.simulation_finished),
        }
    }
}

fn traverse_result<T, E>(iter: Vec<StdResult<T, E>>) -> StdResult<Vec<T>, E> {
    let mut ret = Vec::new();
    for r in iter {
        ret.push(r?);
    }
    Ok(ret)
}

impl ParSimFinished {
    pub fn report(&self) -> ParSimReport {
        // util::save(Path::new("fin_par_sim.json"), self);
        let outputs: Vec<SingSimReport> =
            self.outputs.iter().map(SingSimFinished::report).collect();
        let total_cpu_time = outputs
            .iter()
            .map(|o| o.total_cpu_time.clone())
            .fold(Omitable::Available(0.), |t1, t2| {
                Omitable::map2(|x, y| x + y, t1, t2)
            });

        let simulation_finished = outputs
            .iter()
            .map(|o| o.simulation_finished.clone())
            .fold(Omitable::Available(true), |t1, t2| {
                Omitable::map2(|x, y| x & y, t1, t2)
            });

        let dose = Omitable::from_result(Self::compute_dose(&outputs));

        // util::save(&Path::new("test_data/asdf.json"),self);

        ParSimReport {
            input: self.input.clone(),
            outputs: Omitable::Available(outputs),
            total_cpu_time,
            simulation_finished,
            dose,
        }
    }

    fn compute_dose(reports: &[SingSimReport]) -> Result<Vec<(String, UncertainF64)>> {
        let doses1: Vec<Result<Vec<(String, UncertainF64)>>> = reports
            .iter()
            .map(|o| o.dose.clone().into_result())
            .collect();
        let doses2 = traverse_result(doses1)?;
        if doses2.is_empty() {
            return Ok(Vec::new());
        };
        let mut ret = doses2[0].clone();
        let nruns = doses2.len();
        for i_run in 1..nruns {
            if doses2[i_run].len() != ret.len() {
                let msg = "Simulations have inconsistend 
                    numbers of scoring geometries."
                    .to_string();
                return Err(msg);
            }
            for i_reg in 0..ret.len() {
                let d_new = {
                    let (ref s_inc, ref d_inc) = doses2[i_run][i_reg];
                    let (ref s_ret, ref d_ret) = ret[i_reg];
                    if *s_inc == *s_ret {
                        *d_ret + *d_inc
                    } else {
                        let msg = "Simulation have inconsistent scoring regions
                        "
                            .to_string();
                        Err(msg)?
                    }
                };
                ret[i_reg].1 = d_new;
            }
        }
        let wt = UncertainF64::from_value_var(1. / (nruns as f64), 0.);
        // normalize
        ret = ret.iter()
            .map(|&(ref label, ref dose)| (label.to_string(), *dose * wt))
            .collect();
        Ok(ret)
    }
}

impl ParSimReport {
    pub fn to_string_smart(&self) -> String {
        format!("{}", self)
    }

    pub fn to_string_all(&self) -> String {
        let mut ret = String::new();
        ret.push_str(&Self::string_section("Output"));
        ret.push_str(&self.to_string_input());
        ret.push_str("\n");
        ret.push_str(&Self::string_section("Output"));
        ret.push_str(&self.to_string_output());
        ret
    }

    pub fn string_section(title: &str) -> String {
        format!("\n{:#^width$}\n", " ".to_string() + title + " ", width = 50)
    }

    pub fn to_string_input(&self) -> String {
        self.string_input()
    }

    pub fn to_string_output(&self) -> String {
        let mut ret = String::new();
        ret.push_str(&self.string_total_cpu_time());
        ret.push_str(&"\n");
        ret.push_str(&self.string_simulation_finished());
        ret.push_str(&"\n");
        ret.push_str(&self.string_dose());
        ret.push_str(&"\n");
        ret
    }

    fn string_dose(&self) -> String {
        let mut ret = String::new();
        match self.dose {
            Omitable::Available(ref v) => for &(ref name, score) in v {
                let value = score.value();
                let pstd = score.rstd() * 100.;
                ret.push_str(&format!("{}: {} +- {}%\n", name, value, pstd));
            },
            Omitable::Omitted => {}
            Omitable::Fail(ref s) => ret.push_str(&format!("{}", s)),
        };
        ret
    }

    fn string_key_omittable<T: fmt::Display>(key: &str, val: &Omitable<T>) -> String {
        match val {
            &Omitable::Available(ref t) => format!("{}: {}", key, t),
            &Omitable::Omitted => "".to_string(),
            &Omitable::Fail(ref msg) => format!("{}: {}", key, msg),
        }
    }

    fn string_total_cpu_time(&self) -> String {
        Self::string_key_omittable("Total cpu time", &self.total_cpu_time)
    }

    fn string_simulation_finished(&self) -> String {
        Self::string_key_omittable("Simulation finished", &self.simulation_finished)
    }

    fn string_input(&self) -> String {
        format!("{}", self.input.prototype)
    }
}

impl fmt::Display for ParSimReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.to_string_all())
    }
}

impl fmt::Display for SingSimInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.input_content)?;
        writeln!(f, "Application: {}", self.application)?;
        writeln!(f, "Pegsfile: {}", self.pegsfile)?;
        write!(f, "Checksum: {}", self.checksum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use util::{asset_path, load};
    use uncertain::UncertainF64;

    #[test]
    fn test_report_par_sim() {
        let path = asset_path().join("fin_par_sim.json");
        let raw: ParSimFinished = load(&path).unwrap();
        let report: ParSimReport = raw.report();
        // println!("{}", report);

        let dose1 = UncertainF64::from_value_rstd(1.2027e-14, 6.940 / 100.);
        let dose2 = UncertainF64::from_value_rstd(1.1735e-14, 6.850 / 100.);
        let dose3 = UncertainF64::from_value_rstd(1.3713e-14, 7.010 / 100.);
        let dose4 = UncertainF64::from_value_rstd(1.3646e-14, 6.552 / 100.);
        let dose5 = UncertainF64::from_value_rstd(1.2904e-14, 6.927 / 100.);
        let dose6 = UncertainF64::from_value_rstd(1.2592e-14, 7.217 / 100.);
        let dose7 = UncertainF64::from_value_rstd(1.1982e-14, 6.917 / 100.);
        let dose8 = UncertainF64::from_value_rstd(1.2596e-14, 7.158 / 100.);
        let dose_combined = UncertainF64::from_value_var(1. / 8., 0.)
            * (dose1 + dose2 + dose3 + dose4 + dose5 + dose6 + dose7 + dose8);

        let dose_reported = report.dose.into_result().unwrap().first().unwrap().1;
        assert_relative_eq!(dose_reported.value(), dose_combined.value());
        assert_relative_eq!(dose_reported.rstd(), dose_combined.rstd());
    }
}
