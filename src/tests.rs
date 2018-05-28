#[cfg(test)]
mod tests {
    use util::{asset_path, load};
    use std::path::Path;
    use simulation::ParSimReport;
    use assert_cli;
    use rand;
    use rand::Rng;
    use std::fs;
    use tempfile::{tempdir};

    fn randstring() -> String {
        let ret: String = rand::thread_rng().gen_ascii_chars().take(10).collect();
        ret
    }

    fn run_and_load(input_path:&Path, extra_args:&[&str]) -> ParSimReport {
        let sinput_path = input_path.to_str().unwrap();
        let output_dir = tempdir().unwrap();
        let output_path = output_dir.path().join("out.json");
        let soutput_path = output_path.to_str().unwrap();
        let mut args = vec!["run", sinput_path];
        args.extend(extra_args.iter());
        args.extend(["-o", soutput_path].iter());
        assert_cli::Assert::main_binary()
            .with_args(&args)
            .unwrap();
        let r: ParSimReport = load(&output_path).unwrap();
        r
    }

    #[test]
    fn test_run_umlauts() {
        let input_path = asset_path().join("umlauts.egsinp");
        let _r = run_and_load(&input_path, &[]);
    }

    #[test]
    fn test_run_multiple_geometries() {
        let input_path = asset_path().join("block2.egsinp");
        let sinput_path = input_path.to_str().unwrap();
        let output_path = tempdir().unwrap().path().join(randstring());
        let soutput_path = output_path.to_str().unwrap();
        assert_cli::Assert::main_binary()
            .with_args(&["run", sinput_path, "-o", soutput_path])
            .stdout().contains("finishSimulation(egs_chamber) 0")
            .unwrap();

        let r: ParSimReport = load(&output_path).unwrap();
        let doses = r.dose.into_result().unwrap();
        assert_eq!(doses.len(), 3);
        let (ref geo0, dose0) = doses[0];
        let (ref geo1, dose1) = doses[1];
        let (ref geo01, dose01) = doses[2];
        assert_eq!(geo0, "the_cylinder");
        assert_eq!(geo1, "the_cylinder");
        assert_eq!(geo01, "the_cylinder");
        assert!(((dose0.value() + dose1.value() / dose01.value()).abs() - 1.) < 0.01);
    }

    #[test]
    fn test_run_custom_seeds_ncases() {
        let input_path = asset_path().join("block2.egsinp");
        let sinput_path = input_path.to_str().unwrap();
        let output_path = tempdir().unwrap().path().join("output").join(randstring());
        let soutput_path = output_path.to_str().unwrap();
        assert_cli::Assert::main_binary()
            .with_args(&["run", sinput_path,
                       "-o", soutput_path,
                       "--seeds=[[1983,324],[3,4]]",
                       "--ncases=[173, 200]"])
            .stdout().contains("finishSimulation(egs_chamber) 0")
            .unwrap();

        let r: ParSimReport = load(&output_path).unwrap();
        assert_eq!(r.input.seeds,vec![(1983,324),(3,4)]);
        assert_eq!(r.input.ncases,vec![173, 200]);
        let outs = r.outputs.into_result().unwrap();
        let s:String = outs[0].clone().input.into_result().unwrap()
            .content;
        assert!(s.contains("173"));
        assert!(s.contains("1983 324"));
    }

    #[test]
    fn test_run_bad_pegs() {
        let input_path = asset_path().join("block2.egsinp");
        let sinput_path = input_path.to_str().unwrap();
        let output_path = tempdir().unwrap().path().join(randstring());
        let soutput_path = output_path.to_str().unwrap();
        assert_cli::Assert::main_binary()
            .with_args(&["run", sinput_path, "-o", soutput_path, "-p", "tutor_data"])
            .stdout().contains("PROGRAM STOPPED IN HATCH BECAUSE THE")
            .unwrap();

        let _r: ParSimReport = load(&output_path).unwrap();
    }

    #[test]
    fn test_run_many() {
        let input_path1 = asset_path().join("input_many").join("file1.egsinp");
        let input_path2 = asset_path().join("input_many").join("file2.egsinp");

        let r1 = run_and_load(&input_path1, &[]);
        let r2 = run_and_load(&input_path2, &[]);

        let input_path_many = asset_path().join("input_many");
        let sinput_path_many = input_path_many.to_str().unwrap();
        let output_dir_many = tempdir().unwrap();
        let output_path_many1 = output_dir_many.path().join("file1.json");
        let output_path_many2 = output_dir_many.path().join("file2.json");
        let soutput_path_many = output_dir_many.path().to_str().unwrap();

        assert_cli::Assert::main_binary()
            .with_args(&["run", sinput_path_many, "-o", soutput_path_many])
            .unwrap();

        let m1: ParSimReport = load(&output_path_many1).expect("output_path_many1");
        let m2: ParSimReport = load(&output_path_many2).unwrap();
        assert_eq!(r1.dose, m1.dose);
        assert_eq!(r2.dose, m2.dose);
        assert!(r1 != m1);
    }

    #[test]
    fn test_rerun() {
        let input_path = asset_path().join("block.egsinp");
        let input_path = input_path.to_str().unwrap();
        // let output_dir = asset_path().join("output");
        let output_dir = tempdir().unwrap();
        let output_path1 = output_dir.path().join(randstring());
        let soutput_path1 = output_path1.to_str().unwrap();
        let output_path2 = output_dir.path().join(randstring());
        let soutput_path2 = output_path2.to_str().unwrap();
        assert_cli::Assert::main_binary()
            .with_args(&["run", input_path, "-o", soutput_path1])
            .unwrap();

        assert_cli::Assert::main_binary()
            .with_args(&["rerun", soutput_path1, "-o", soutput_path2])
            .unwrap();

        let r1: ParSimReport = load(&output_path1).unwrap();
        let r2: ParSimReport = load(&output_path2).unwrap();

        assert!(r1 != r2);
        assert_eq!(r1.dose, r2.dose);
        r1.dose.into_result().unwrap();
    }


}
