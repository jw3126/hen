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
use uncertain::Uf64;
use output_parser;
use std::fmt;
use errors::*;
use util;
use std::result::Result as StdResult;
use itertools::Itertools;
use omittable::Omittable;

pub type Seed = (usize, usize); // is this correct integer type?

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingSimInput {
    pub application: String,
    pub content: String,
    pub pegsfile: String,
    pub checksum: String,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingSimInputBuilder {
    application: Option<String>,
    content: Option<String>,
    pegsfile: Option<String>,
    filename: Option<String>,
}

impl SingSimInputBuilder {
    pub fn new() -> Self {
        SingSimInputBuilder {
            application: None,
            content: None,
            pegsfile: None,
            filename: None,
        }
    }

    pub fn application(mut self, app: &str) -> Self {
        self.application = Some(app.to_string());
        self
    }

    pub fn content(mut self, s: &str) -> Self {
        self.content = Some(s.to_string());
        self
    }

    pub fn filename(mut self, s: &str) -> Self {
        self.filename = Some(s.to_string());
        self
    }

    pub fn pegsfile(mut self, s: &str) -> Self {
        self.pegsfile = Some(s.to_string());
        self
    }

    fn get_checksum(&self) -> Option<String> {
        if let &Some(ref content) = &self.content {
            let digest = sha3::Sha3_256::digest(content.as_bytes());
            let checksum = format!("{:x}", digest);
            Some(checksum)
        } else {
            None
        }
    }

    pub fn build(self) -> Result<SingSimInput> {
        let checksum = (&self)
            .get_checksum()
            .ok_or("Cannot compute checksum. Are all fields of builder set?")?;

        match self {
            SingSimInputBuilder {
                application: Some(application),
                content: Some(content),
                pegsfile: Some(pegsfile),
                filename: Some(filename),
            } => {
                let sim = SingSimInput {
                    application,
                    content,
                    pegsfile,
                    checksum,
                    filename,
                };
                Ok(sim)
            }
            _ => bail!("All fields of builder should be set."),
        }
    }
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
pub struct SingSimParsedOutput {
    pub dose: StubResult<Vec<(String, Uf64)>>,
    pub total_cpu_time: StubResult<f64>,
    pub simulation_finished: StubResult<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SingSimReport {
    pub input: Omittable<SingSimInput>,
    pub stderr: Omittable<String>,
    pub stdout: Omittable<String>,
    pub exit_status: Omittable<i32>,
    pub dose: Omittable<Vec<(String, Uf64)>>,
    pub total_cpu_time: Omittable<f64>,
    pub simulation_finished: Omittable<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParSimFinished {
    pub input: ParSimInput,
    pub outputs: Vec<SingSimFinished>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParSimReport {
    pub input: ParSimInput,
    pub single_runs: Vec<SingSimReport>,

    pub total_cpu_time: Omittable<f64>,
    pub simulation_finished: Omittable<bool>,
    pub dose: Omittable<Vec<(String, Uf64)>>,
}

impl ParSimInput {
    pub fn run(&self) -> Result<ParSimFinished> {
        let cleanup = true;
        self.run_with_cleanup_option(cleanup)
    }

    pub fn run_with_cleanup_option(&self, cleanup: bool) -> Result<ParSimFinished> {
        self.validate()?;
        let stream = TokenStream::parse_string(&(self.prototype.content))?;
        let streams = stream.split(&self.seeds, &self.ncases)?;
        let application = &self.prototype.application;
        let pegsfile = &self.prototype.pegsfile;
        let compute_single_output = |content: String| {
            let sim = SingSimInputBuilder::new()
                .application(application)
                .content(&content)
                .pegsfile(pegsfile)
                .filename(&self.prototype.filename)
                .build()
                .unwrap();
            if cleanup {
                sim.run_and_cleanup()
            } else {
                sim.run()
            }
        };

        let outputs: Vec<SingSimFinished> = streams
            .par_iter()
            .map(TokenStream::to_string)
            .map(compute_single_output)
            .collect();

        let ret = ParSimFinished {
            input: self.clone(),
            outputs,
        };
        Ok(ret)
    }

    pub fn validate(&self) -> Result<()> {
        let len_seeds = self.seeds.len();
        let len_ncases = self.ncases.len();
        if len_seeds != len_ncases {
            bail!("Got {} seeds, but {} ncases", len_seeds, len_ncases);
        }

        if !util::has_unique_elements(self.seeds.clone()) {
            bail!("Duplicate seeds {:?}", self.seeds);
        }

        Ok(())
    }

    fn validate_combine(inps: &[ParSimInput]) -> Result<()> {
        if inps.is_empty() {
            bail!("Cannot combine empty collection of simulations.");
        }
        let checksums: Vec<String> = inps.iter()
            .map(|inp| inp.prototype.checksum.clone())
            .collect();
        if !checksums.iter().all_equal() {
            bail!(
                "Cannot combine simulations with different checksums: {:?}",
                checksums
            );
        }
        Ok(())
    }

    pub fn combine(inps: &[ParSimInput]) -> Result<ParSimInput> {
        ParSimInput::validate_combine(inps)?;
        let prototype = inps[0].prototype.clone();
        let mut seeds = Vec::new();
        let mut ncases = Vec::new();
        for inp in inps {
            ncases.extend(&inp.ncases);
            seeds.extend(&inp.seeds);
        }
        let ret = ParSimInput {
            prototype,
            seeds,
            ncases,
        };
        ret.validate()?;
        Ok(ret)
    }
}

fn egs_home_path() -> PathBuf {
    let mut path = PathBuf::new();
    let segs_home = std::env::var("EGS_HOME").expect("Cannot find EGS_HOME");
    path.push(segs_home);
    path
}

impl SingSimInput {
    pub fn from_egsinp_path(application: &str, path: &Path, pegsfile: &str) -> Result<Self> {
        let mut file = File::open(path).chain_err(|| cannot_read(&path))?;
        let mut content = String::new();
        let filename = path.file_name()
            .ok_or("Error getting file_name")?
            .to_str()
            .unwrap();
        file.read_to_string(&mut content)
            .chain_err(|| cannot_read(&path))?;
        let sim = SingSimInputBuilder::new()
            .pegsfile(pegsfile)
            .filename(filename)
            .application(application)
            .content(&content)
            .build()
            .unwrap();
        Ok(sim)
    }

    fn run_cmd(&self) -> std::io::Result<Output> {
        let mut file = fs::File::create(self.path_exec_with_ext("egsinp"))?;
        file.write_all(self.content.as_bytes()).unwrap();

        let ret = Command::new(self.application.clone())
            .args(&["-i", self.checksum.as_str(), "-p", self.pegsfile.as_str()])
            .output();

        ret
    }

    pub fn run(&self) -> SingSimFinished {
        let out = self.run_cmd().unwrap();
        let ret = SingSimFinished {
            input: self.clone(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            exit_status: out.status.code().unwrap_or(-1),
        };
        ret
    }

    pub fn run_and_cleanup(&self) -> SingSimFinished {
        let ret = self.run();
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

    pub fn split(self, ncases: Vec<u64>, seeds: Vec<Seed>) -> ParSimInput {
        let prototype = self;
        ParSimInput {
            seeds,
            prototype,
            ncases,
        }
    }

    pub fn splitn(self, n: usize) -> Result<ParSimInput> {
        self.split_fancy(None, None, n)
    }

    pub fn split_fancy(
        self,
        mncases: Option<Vec<u64>>,
        mseeds: Option<Vec<Seed>>,
        nthreads: usize,
    ) -> Result<ParSimInput> {
        let stream = TokenStream::parse_string(&(self.content))?;
        let seed_count: Option<usize> = mseeds.as_ref().map(|v| v.len());
        let case_count: Option<usize> = mncases.as_ref().map(|v| v.len());
        let n = seed_count.or(case_count).unwrap_or(nthreads);
        let seeds = mseeds.unwrap_or(stream.generate_seeds(n)?);
        let ncases = mncases.unwrap_or(stream.generate_ncases(n)?);

        Ok(self.split(ncases, seeds))
    }
}

impl SingSimFinished {
    fn parse_output(&self) -> SingSimParsedOutput {
        let mut reader = BufReader::new(self.stdout.as_bytes());
        let rout = output_parser::parse_simulation_output(&mut reader).into_stub();
        match rout {
            Ok(ret) => ret,
            Err(err) => SingSimParsedOutput {
                dose: Err(err.clone()),
                total_cpu_time: Err(err.clone()),
                simulation_finished: Err(err.clone()),
            },
        }
    }

    pub fn report(&self) -> SingSimReport {
        let out = self.parse_output();
        let exit_status = Omittable::Available(self.exit_status);
        let dose = Omittable::from(out.dose);
        let total_cpu_time = Omittable::from(out.total_cpu_time);
        let simulation_finished = Omittable::from(out.simulation_finished);
        let stderr = match simulation_finished {
            Omittable::Available(true) => Omittable::Omitted,
            _ => Omittable::Available(self.stderr.clone()),
        };
        let stdout = match simulation_finished {
            Omittable::Available(true) => Omittable::Omitted,
            _ => Omittable::Available(self.stdout.clone()),
        };
        let input = match simulation_finished {
            Omittable::Available(true) => Omittable::Omitted,
            _ => Omittable::Available(self.input.clone()),
        };
        SingSimReport {
            input,
            stderr,
            stdout,
            exit_status,
            dose,
            total_cpu_time,
            simulation_finished,
        }
    }

    // TODO this is not dry
    pub fn report_full(&self) -> SingSimReport {
        let out = self.parse_output();
        let exit_status = Omittable::Available(self.exit_status);
        let dose = Omittable::from(out.dose);
        let total_cpu_time = Omittable::from(out.total_cpu_time);
        let simulation_finished = Omittable::from(out.simulation_finished);
        let stdout = Omittable::Available(self.stdout.clone());
        let stderr = Omittable::Available(self.stderr.clone());
        let input = Omittable::Available(self.input.clone());
        SingSimReport {
            input,
            stderr,
            stdout,
            exit_status,
            dose,
            total_cpu_time,
            simulation_finished,
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
        // we want the first run to be detailed
        let mut single_runs: Vec<SingSimReport> =
            self.outputs.iter().map(SingSimFinished::report).collect();
        single_runs[0] = self.outputs[0].report_full();

        let input = self.input.clone();
        let total_cpu_time = Omittable::Omitted;
        let simulation_finished = Omittable::Omitted;
        let dose = Omittable::Omitted;
        let ret = ParSimReport {
            input,
            single_runs,
            total_cpu_time,
            simulation_finished,
            dose,
        };
        let ret = ret.recalculate();
        ret
    }
}

fn compute_total_cpu_time(single_runs: &[SingSimReport]) -> Omittable<f64> {
    single_runs
        .iter()
        .map(|o| o.total_cpu_time.clone())
        .fold(Omittable::Available(0.), |t1, t2| {
            Omittable::map2(|x, y| x + y, t1, t2)
        })
}

fn compute_simulation_finished(single_runs: &[SingSimReport]) -> Omittable<bool> {
    single_runs
        .iter()
        .map(|o| o.simulation_finished.clone())
        .fold(Omittable::Available(true), |t1, t2| {
            Omittable::map2(|x, y| x & y, t1, t2)
        })
}

fn compute_dose(single_runs: &[SingSimReport]) -> Omittable<Vec<(String, Uf64)>> {
    Omittable::from(compute_dose_result(&single_runs))
}

fn compute_dose_result(reports: &[SingSimReport]) -> Result<Vec<(String, Uf64)>> {
    let doses1: Vec<StubResult<Vec<(String, Uf64)>>> = reports
        .iter()
        .map(|o| o.dose.clone().into_stub_result())
        .collect();
    let doses2 = traverse_result(doses1)?;
    if doses2.is_empty() {
        return Ok(Vec::new());
    };
    let mut ret = doses2[0].clone();
    let nruns = doses2.len();
    for i_run in 1..nruns {
        if doses2[i_run].len() != ret.len() {
            bail!("Simulations have inconsistend numbers of scoring geometries.");
        }
        for i_reg in 0..ret.len() {
            let d_new = {
                let (ref s_inc, ref d_inc) = doses2[i_run][i_reg];
                let (ref s_ret, ref d_ret) = ret[i_reg];
                if *s_inc == *s_ret {
                    *d_ret + *d_inc
                } else {
                    bail!("Simulation have inconsistent scoring regions");
                }
            };
            ret[i_reg].1 = d_new;
        }
    }
    let wt = Uf64::from_value_var(1. / (nruns as f64), 0.);
    // normalize
    let mask_nan = |dose: Uf64| {
        if dose.rstd().is_finite() {
            dose
        } else {
            Uf64::from_value_rstd(dose.value(), 1.0)
        }
    };
    ret = ret.iter()
        .map(|&(ref label, ref dose)| (label.to_string(), mask_nan(*dose * wt)))
        .collect();
    Ok(ret)
}

impl ParSimReport {
    pub fn recalculate(self) -> Self {
        let ParSimReport {
            input,
            single_runs,
            dose,
            total_cpu_time,
            simulation_finished,
        } = self;
        let _ = dose;
        let _ = total_cpu_time;
        let _ = simulation_finished;
        let dose = compute_dose(&single_runs);
        let total_cpu_time = compute_total_cpu_time(&single_runs);
        let simulation_finished = compute_simulation_finished(&single_runs);
        ParSimReport {
            input,
            single_runs,
            dose,
            total_cpu_time,
            simulation_finished,
        }
    }

    pub fn combine(sims: &[ParSimReport]) -> Result<ParSimReport> {
        let mut inputs = Vec::new();
        let mut single_runs = Vec::new();
        for sim in sims {
            inputs.push(sim.input.clone());
            single_runs.extend(sim.single_runs.clone());
        }
        let input = ParSimInput::combine(&inputs)?;

        let dose = Omittable::Omitted;
        let total_cpu_time = Omittable::Omitted;
        let simulation_finished = Omittable::Omitted;
        let ret = ParSimReport {
            input,
            single_runs,
            dose,
            total_cpu_time,
            simulation_finished,
        };
        let ret = ret.recalculate();
        Ok(ret)
    }

    pub fn to_string_smart(&self) -> String {
        format!("{}", self)
    }

    pub fn to_string_all(&self) -> String {
        let mut ret = String::new();
        ret.push_str(&Self::string_section("Input"));
        ret.push_str("\n");
        ret.push_str(&self.to_string_input());
        ret.push_str("\n");
        ret.push_str(&Self::string_section("Example"));
        ret.push_str("\n");
        ret.push_str(&self.to_string_first_sing_sim());
        ret.push_str("\n");
        ret.push_str(&Self::string_section("Output"));
        ret.push_str("\n");
        ret.push_str(&self.to_string_output());
        ret
    }

    pub fn to_string_first_sing_sim(&self) -> String {
        let v = &self.single_runs;

        if v.len() == 0 {
            "".to_string()
        } else {
            format!("{}", v[0]).to_string()
        }
    }

    pub fn string_section(title: &str) -> String {
        format!("\n{:#^width$}\n", " ".to_string() + title + " ", width = 50)
    }

    pub fn to_string_input(&self) -> String {
        self.string_input()
    }

    pub fn compute_efficiency(&self) -> Omittable<f64> {
        fn inner(doses: Vec<(String, Uf64)>, t: f64) -> f64 {
            let mut ret = 0.;
            let n = doses.len();
            for (ref _label, ref score) in doses {
                let rvar: f64 = score.rvar();
                ret += 1.0 / rvar / t;
            }
            ret / n as f64
        };
        Omittable::map2(inner, self.dose.clone(), self.total_cpu_time.clone())
    }

    pub fn to_string_output(&self) -> String {
        let mut ret = String::new();
        ret.push_str(&self.string_total_cpu_time());
        ret.push_str(&"\n");
        ret.push_str(&self.string_simulation_finished());
        ret.push_str(&"\n");
        ret.push_str(&self.string_dose());
        ret.push_str(&"\n");
        ret.push_str(&self.string_efficienty());
        ret.push_str(&"\n");
        ret
    }

    fn string_dose(&self) -> String {
        let mut ret = String::new();
        match self.dose {
            Omittable::Available(ref v) => for &(ref name, score) in v {
                let value = score.value();
                let pstd = score.rstd() * 100.;
                ret.push_str(&format!("{}: {} +- {}%\n", name, value, pstd));
            },
            Omittable::Omitted => {}
            Omittable::Fail(ref s) => ret.push_str(&format!("{}", s)),
        };
        ret
    }

    fn string_key_omittable<T: fmt::Display>(key: &str, val: &Omittable<T>) -> String {
        match val {
            &Omittable::Available(ref t) => format!("{}: {}", key, t),
            &Omittable::Omitted => "".to_string(),
            &Omittable::Fail(ref msg) => format!("{}: {}", key, msg),
        }
    }

    fn string_total_cpu_time(&self) -> String {
        Self::string_key_omittable("Total cpu time", &self.total_cpu_time)
    }

    fn string_efficienty(&self) -> String {
        Self::string_key_omittable("Efficiency", &self.compute_efficiency())
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

impl fmt::Display for SingSimReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.input)?;
        writeln!(f, "{}", self.stdout)?;
        writeln!(f, "{}", self.stderr)
    }
}

impl fmt::Display for SingSimInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.content)?;
        writeln!(f, "Filename: {}", self.filename)?;
        writeln!(f, "Application: {}", self.application)?;
        writeln!(f, "Pegsfile: {}", self.pegsfile)?;
        write!(f, "Checksum: {}", self.checksum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use util::{asset_path, load};
    use uncertain::Uf64;

    #[test]
    fn test_report_par_sim() {
        let path = asset_path().join("fin_par_sim.json");
        let raw: ParSimFinished = load(&path).unwrap();
        let report: ParSimReport = raw.report();
        // println!("{}", report);

        let dose1 = Uf64::from_value_rstd(1.2027e-14, 6.940 / 100.);
        let dose2 = Uf64::from_value_rstd(1.1735e-14, 6.850 / 100.);
        let dose3 = Uf64::from_value_rstd(1.3713e-14, 7.010 / 100.);
        let dose4 = Uf64::from_value_rstd(1.3646e-14, 6.552 / 100.);
        let dose5 = Uf64::from_value_rstd(1.2904e-14, 6.927 / 100.);
        let dose6 = Uf64::from_value_rstd(1.2592e-14, 7.217 / 100.);
        let dose7 = Uf64::from_value_rstd(1.1982e-14, 6.917 / 100.);
        let dose8 = Uf64::from_value_rstd(1.2596e-14, 7.158 / 100.);
        let dose_combined = Uf64::from_value_var(1. / 8., 0.)
            * (dose1 + dose2 + dose3 + dose4 + dose5 + dose6 + dose7 + dose8);

        let dose_reported = report.dose.into_stub_result().unwrap().first().unwrap().1;
        assert_relative_eq!(dose_reported.value(), dose_combined.value());
        assert_relative_eq!(dose_reported.rstd(), dose_combined.rstd());
    }
}
