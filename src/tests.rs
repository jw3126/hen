#[cfg(test)]
mod tests {
    use super::*;
    use util::{load, asset_path};
    use simulation::{ParallelSimulationReport};
    use assert_cli;

    #[test]
    fn rerun() {
        let input_path = asset_path().join("block.egsinp");
        let input_path = input_path.to_str().unwrap();
        let output_path1 =  asset_path().join("output").join("out1.json");
        let soutput_path1 = output_path1.to_str().unwrap();
        let output_path2 =  asset_path().join("output").join("out2.json");
        let soutput_path2 = output_path2.to_str().unwrap();
        assert_cli::Assert::main_binary()
            .with_args(&["run", input_path, "-o", soutput_path1])
            .unwrap();


        assert_cli::Assert::main_binary()
            .with_args(&["rerun", soutput_path1, "-o", soutput_path2])
            .unwrap();

        let r1:ParallelSimulationReport = load(&output_path1).unwrap();
        let r2:ParallelSimulationReport = load(&output_path2).unwrap();

        assert!(r1 != r2);
        assert_eq!(r1.dose, r2.dose);
    }
}
