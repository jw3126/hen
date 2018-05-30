use clap::ArgMatches;
use std::path::{Path,PathBuf};
use std::env::current_dir;
use std;
use app::util::SubCmd;
use util::{Result,read_paths_in_dir};
use std::collections::HashMap;
use simulation::ParSimReport;
use app::util::GetMatch;
use util::{load,save};

#[derive(Debug)]
struct CombineConfig {
    inputpath: PathBuf,
    outputpath:PathBuf,
}

impl CombineConfig {
    fn compute_output_path(&self, input_path:&Path) -> Result<PathBuf> {
            let stem = input_path.file_stem()
                .ok_or(format!("Cannot get stem of {:?}", &input_path))?;
            let mut opath = self.outputpath.join(stem);
            opath.set_extension("henout");
            Ok(opath)
    }

    fn compute_input_paths(&self) -> Result<Vec<PathBuf>> {
        let ret = read_paths_in_dir(&self.inputpath)?
                .iter()
                .filter(|path|path.extension()
                        .and_then(|ext|ext.to_str())
                        .unwrap_or("fail") == "henout")
                .map(|p|p.clone())
                .collect();
        Ok(ret)
    }

    fn create_path_report_dict(&self) ->
        Result<HashMap<PathBuf,Vec<ParSimReport>>> {
        let paths = self.compute_input_paths()?;
        let mut ret = HashMap::new();
        for ipath in paths {
            let opath = self.compute_output_path(&ipath)?;
            let rep = load(&ipath)?;
            ret.entry(opath).or_insert(Vec::new()).push(rep);
        }
        Ok(ret)
    }
}

impl SubCmd for CombineConfig{
    fn parse(m:&ArgMatches) -> Result<Self> {
        let inputpath = m.get_abspath("INPUT")?;
        let outputpath = m.get_abspath("OUTPUT")?;
        let ret = CombineConfig {inputpath, outputpath};
        Ok(ret)
    }

    fn run(&self) -> Result<()> {
        let d = self.create_path_report_dict()?;
        for (output_path, sims) in &d {
            let out = ParSimReport::combine(&sims)?;
            save(&output_path, &out)?
        }

        Ok(())
    }
}
