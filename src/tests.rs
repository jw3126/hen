use util::{asset_path, has_unique_elements, load};
use std::path::Path;
use simulation::{ParSimInput, ParSimReport, Seed};
use assert_cli;
use rand;
use rand::Rng;
use std::fs;
use tempfile::tempdir;
use uncertain::UncertainF64;

fn randstring() -> String {
    let ret: String = rand::thread_rng().gen_ascii_chars().take(10).collect();
    ret
}

fn run_and_load(input_path: &Path, extra_args: &[&str]) -> ParSimReport {
    let sinput_path = input_path.to_str().unwrap();
    let output_dir = tempdir().unwrap();
    let output_path = output_dir.path().join("out.json");
    let soutput_path = output_path.to_str().unwrap();
    let mut args = vec!["run", sinput_path];
    args.extend(extra_args.iter());
    args.extend(["-o", soutput_path].iter());
    assert_cli::Assert::main_binary().with_args(&args).unwrap();
    let r: ParSimReport = load(&output_path).unwrap();
    r
}

fn assert_close_doses(dose1: Vec<(String, UncertainF64)>, dose2: Vec<(String, UncertainF64)>) {
    if dose1.len() != dose2.len() {
        panic!("Vectors of different length");
    }
    for (&(ref s1, x1), &(ref s2, x2)) in dose1.iter().zip(dose2.iter()) {
        if s1 != s2 {
            panic!("Names of dose regions must match");
        }
        assert_relative_eq!(x1.value(), x2.value());
        assert_relative_eq!(x1.std(), x2.std());
    }
}

#[test]
fn test_run_umlauts() {
    let input_path = asset_path().join("umlauts.egsinp");
    let _r = run_and_load(&input_path, &[]);
}

#[test]
fn test_run_multiple_geometries() {
    let input_path = asset_path().join("three_calc_geos.egsinp");
    let sinput_path = input_path.to_str().unwrap();
    let output_path = tempdir().unwrap().path().join(randstring());
    let soutput_path = output_path.to_str().unwrap();
    assert_cli::Assert::main_binary()
        .with_args(&["run", sinput_path, "-o", soutput_path])
        .stdout()
        .contains("finishSimulation(egs_chamber) 0")
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
    assert!(((dose0.value() + dose1.value() / dose01.value()).abs() - 1.) < 0.03);
}

#[test]
fn test_run_custom_seeds_ncases() {
    let input_path = asset_path().join("three_calc_geos.egsinp");
    let sinput_path = input_path.to_str().unwrap();
    let output_path = tempdir().unwrap().path().join("output").join(randstring());
    let soutput_path = output_path.to_str().unwrap();
    assert_cli::Assert::main_binary()
        .with_args(&[
            "run",
            sinput_path,
            "-o",
            soutput_path,
            "--seeds=[[1983,324],[3,4]]",
            "--ncases=[173, 200]",
        ])
        .stdout()
        .contains("finishSimulation(egs_chamber) 0")
        .unwrap();

    let r: ParSimReport = load(&output_path).unwrap();
    assert_eq!(r.input.seeds, vec![(1983, 324), (3, 4)]);
    assert_eq!(r.input.ncases, vec![173, 200]);
    let outs = r.single_runs;
    let s: String = outs[0].clone().input.into_result().unwrap().content;
    assert!(s.contains("173"));
    assert!(s.contains("1983 324"));
}

#[test]
fn test_run_bad_pegs() {
    let input_path = asset_path().join("three_calc_geos.egsinp");
    let sinput_path = input_path.to_str().unwrap();
    let output_path = tempdir().unwrap().path().join(randstring());
    let soutput_path = output_path.to_str().unwrap();
    assert_cli::Assert::main_binary()
        .with_args(&[
            "run",
            sinput_path,
            "-o",
            soutput_path,
            "-p",
            "tutor_data",
            "-t4",
        ])
        .stdout()
        .contains("PROGRAM STOPPED IN HATCH BECAUSE THE")
        .unwrap();

    let r: ParSimReport = load(&output_path).unwrap();
    let runs = r.single_runs;
    assert_eq!(runs.len(), 4);
    // no output should be discarded in case of problem
    assert!(runs.iter().all(|r| r.stdout.is_available()));
    assert!(runs.iter().all(|r| r.stderr.is_available()));
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
    let output_path_many1 = output_dir_many.path().join("file1.henout");
    let output_path_many2 = output_dir_many.path().join("file2.henout");
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
    let input_path = asset_path().join("three_calc_geos.egsinp");
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

#[test]
fn test_split() {
    let input_path = asset_path().join("three_calc_geos.egsinp");
    let sinput_path = input_path.to_str().unwrap();
    let output_dir = tempdir().unwrap();
    let output_dir = output_dir.path();
    let soutput_dir = output_dir.to_str().unwrap();
    assert_cli::Assert::main_binary()
        .with_args(&[
            "split",
            sinput_path,
            "-o",
            soutput_dir,
            "--nthreads",
            "2",
            "--nfiles",
            "3",
        ])
        .unwrap();
    let output_paths = fs::read_dir(output_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path());
    let files: Vec<ParSimInput> = output_paths.map(|p| load(&p).unwrap()).collect();
    assert_eq!(files.len(), 3);
    assert_eq!(files[0].prototype, files[1].prototype);
    assert_eq!(files[0].prototype, files[2].prototype);
    let mut seeds: Vec<Seed> = Vec::new();
    let mut ncases: Vec<u64> = Vec::new();
    for file in files {
        seeds.extend(&file.seeds);
        ncases.extend(&file.ncases);
    }
    assert_eq!(seeds.len(), 6);
    assert_eq!(ncases.len(), 6);
    let ncase_expected = 1000;
    let ncase_sum: u64 = ncases.iter().sum();
    assert!(ncase_sum <= ncase_expected);
    assert!(ncase_sum >= ncase_expected - 6);
    assert!(has_unique_elements(seeds));
}

#[test]
fn test_split_run_combine() {
    let input_path = asset_path().join("three_calc_geos.egsinp");
    let sinput_path = input_path.to_str().unwrap();
    let split_dir = tempdir().unwrap();
    let split_dir = split_dir.path();
    let ssplit_dir = split_dir.to_str().unwrap();
    let output_dir = tempdir().unwrap();
    let output_dir = output_dir.path();
    let soutput_dir = output_dir.to_str().unwrap();
    let run_dir = tempdir().unwrap();
    let run_dir = run_dir.path();
    let srun_dir = run_dir.to_str().unwrap();
    assert_cli::Assert::main_binary()
        .with_args(&[
            "split",
            sinput_path,
            "-o",
            ssplit_dir,
            "--nthreads",
            "2",
            "--nfiles",
            "3",
        ])
        .unwrap();
    assert_cli::Assert::main_binary()
        .with_args(&["run", ssplit_dir, "-o", srun_dir])
        .unwrap();
    assert_cli::Assert::main_binary()
        .with_args(&["combine", srun_dir, "-o", soutput_dir])
        .unwrap();

    let output_paths = fs::read_dir(output_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path());
    let files: Vec<ParSimReport> = output_paths.map(|p| load(&p).unwrap()).collect();
    assert_eq!(files.len(), 1);
    let rep_combined = files[0].clone();
    let rep_single = run_and_load(&input_path, &["-t6"]);
    assert!(rep_combined != rep_single);
    assert_close_doses(
        rep_combined.dose.into_result().unwrap(),
        rep_single.dose.into_result().unwrap(),
    );
}
