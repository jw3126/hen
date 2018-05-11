#[cfg(test)]
mod tests {
    use util::{asset_path, load};
    use simulation::ParallelSimulationReport;
    use assert_cli;
    use rand;
    use rand::Rng;
    use std::fs;

    fn randstring() -> String {
        let ret: String = rand::thread_rng().gen_ascii_chars().take(10).collect();
        ret
    }

    #[test]
    fn test_run() {
        let input_path = asset_path().join("block2.egsinp");
        let sinput_path = input_path.to_str().unwrap();
        let output_path = asset_path().join("output").join(randstring());
        let soutput_path = output_path.to_str().unwrap();
        assert_cli::Assert::main_binary()
            .with_args(&["run", sinput_path, "-o", soutput_path])
            .unwrap();
        let r: ParallelSimulationReport = load(&output_path).unwrap();
        let doses = r.dose.into_result().unwrap();
        assert_eq!(doses.len(), 3);
        let (ref geo0, dose0) = doses[0];
        let (ref geo1, dose1) = doses[1];
        let (ref geo01, dose01) = doses[2];
        assert_eq!(geo0, "the_cylinder");
        assert_eq!(geo1, "the_cylinder");
        assert_eq!(geo01, "the_cylinder");
        assert!(((dose0.value() + dose1.value() / dose01.value()).abs() - 1.) < 0.01);
        fs::remove_file(&output_path).unwrap();
    }

    #[test]
    fn test_rerun() {
        let input_path = asset_path().join("block.egsinp");
        let input_path = input_path.to_str().unwrap();
        let output_path1 = asset_path().join("output").join(randstring());
        let soutput_path1 = output_path1.to_str().unwrap();
        let output_path2 = asset_path().join("output").join(randstring());
        let soutput_path2 = output_path2.to_str().unwrap();
        assert_cli::Assert::main_binary()
            .with_args(&["run", input_path, "-o", soutput_path1])
            .unwrap();

        assert_cli::Assert::main_binary()
            .with_args(&["rerun", soutput_path1, "-o", soutput_path2])
            .unwrap();

        let r1: ParallelSimulationReport = load(&output_path1).unwrap();
        let r2: ParallelSimulationReport = load(&output_path2).unwrap();

        assert!(r1 != r2);
        assert_eq!(r1.dose, r2.dose);
        fs::remove_file(&output_path1).unwrap();
        fs::remove_file(&output_path2).unwrap();
    }
}
