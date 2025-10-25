use std::{
    fs::{self},
    path::PathBuf,
    process::{self, Command, Output},
};

use clap::Parser;
use serde_json::Value;

const DEFAULT_MIN_LINE_COVERAGE: f32 = 1.0;
const DEFAULT_MIN_BRANCH_COVERAGE: f32 = 1.0;

const PROFDATA_DIR: &'static str = ".profdata";
const PROFDATA_PATH: &'static str = ".profdata/unittest.profdata";

#[derive(Debug, Parser)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(long)]
    min_line_coverage: Option<f32>,
    #[arg(long)]
    min_branch_coverage: Option<f32>,
    #[arg(help("Path to Cargo project. Defaults to current working directory"))]
    project_dir: Option<String>,
}

#[derive(Debug)]
struct Report {
    line_coverage: f32,
    branch_coverage: f32,
}

fn main() {
    let args = Args::parse();

    let min_line_coverage = args.min_line_coverage.unwrap_or(DEFAULT_MIN_LINE_COVERAGE);
    let min_branch_coverage = args
        .min_branch_coverage
        .unwrap_or(DEFAULT_MIN_BRANCH_COVERAGE);
    let project_dir = args.project_dir.unwrap_or(".".to_string());

    run_test_with_profiling(&project_dir);

    generate_profdata(&project_dir);

    let objects = get_objects(&project_dir);

    let report = execute_report(&project_dir, &objects);

    if report.line_coverage < min_line_coverage {
        eprintln!(
            "Line coverage requirement not met ({} < {})",
            &report.line_coverage, &min_line_coverage
        );
        process::exit(1)
    }
    if report.branch_coverage < min_branch_coverage {
        eprintln!(
            "Branch coverage requirement not met ({} < {})",
            &report.line_coverage, &min_line_coverage
        );
        process::exit(1)
    }
    println!("SUCCESS - All coverage requirements met");
}

fn run_test_with_profiling(project_dir: &String) {
    let cmd = Command::new("cargo")
        .arg("test")
        .env("RUSTFLAGS", "-C instrument-coverage")
        .current_dir(project_dir)
        .output()
        .expect("failed to run cargo test");

    panic_on_fail("cargo test failed", &cmd);
}

fn generate_profdata(project_dir: &String) {
    clear_profdata(project_dir);
    let cmd = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "rust-profdata merge -sparse default_*.profraw -o {}",
            PathBuf::from_iter([project_dir.as_str(), PROFDATA_PATH])
                .to_str()
                .unwrap()
        ))
        .current_dir(project_dir)
        .output()
        .expect("failed to run rust-profdata");
    panic_on_fail("rust-profdata failed", &cmd);
    clear_profraw(project_dir);
}

fn clear_profdata(project_dir: &String) {
    let profdata_dir = PathBuf::from_iter([project_dir.as_str(), PROFDATA_DIR]);
    if fs::exists(&profdata_dir).unwrap() {
        fs::remove_dir_all(&profdata_dir).expect("failed to clean profdata dir");
    }
    fs::create_dir(&profdata_dir).unwrap();
}

fn clear_profraw(project_dir: &String) {
    let files = fs::read_dir(project_dir)
        .expect(format!("unable to list files in {}", project_dir).as_str());
    for file in files {
        let file = file.expect(format!("unable to stat file in {}", project_dir).as_str());
        if !file.file_type().unwrap().is_file() {
            continue;
        }
        let file_name = file.file_name().into_string().unwrap();
        if file_name.starts_with("default") && file_name.ends_with(".profraw") {
            fs::remove_file(PathBuf::from_iter([
                project_dir.as_str(),
                file_name.as_str(),
            ]))
            .unwrap();
        }
    }
}

fn get_objects(project_dir: &String) -> Vec<String> {
    let cmd = Command::new("cargo")
        .arg("test")
        .arg("--no-run")
        .arg("--message-format=json")
        .env("RUSTFLAGS", "-C instrument-coverage")
        .current_dir(project_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute cargo command: {:?}", e));

    panic_on_fail("cargo command failed", &cmd);

    let stdout = String::from_utf8(cmd.stdout).expect("Cargo output was not UTF-8");
    let mut objects: Vec<String> = vec![];
    for line in stdout.lines() {
        let target: Value = serde_json::from_str(line).expect("Unable to parse output as JSON");
        if let Some(test) = target["profile"]["test"].as_bool()
            && test
        {
            let object_paths = target["filenames"]
                .as_array()
                .expect("filenames was not an array");
            for object_path in object_paths {
                objects.push(
                    object_path
                        .as_str()
                        .expect("filename was not a string")
                        .to_string(),
                );
            }
        }
    }
    objects
}

fn execute_report(project_dir: &String, objects: &Vec<String>) -> Report {
    let mut cmd = Command::new("rust-cov");
    cmd.arg("report")
        .arg("--use-color")
        .arg("--show-region-summary=false")
        .arg("--ignore-filename-regex='/.cargo/registry'")
        .arg("-instr-profile")
        .arg(PROFDATA_PATH);

    for object in objects {
        cmd.arg("--object").arg(object.as_str());
    }

    let output = cmd
        .current_dir(project_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute rust-cov: {:?}", e));

    panic_on_fail("rust-cov failed", &output);

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    println!("{}", &stdout);
    println!("{}", &stderr);

    let coverage_line = find_coverage_line(&stdout);
    let coverage_line_parts: Vec<&str> = coverage_line.split_whitespace().collect();
    let line_coverage_str = coverage_line_parts[6].to_string();
    let branch_coverage_str = coverage_line_parts[9].to_string();

    Report {
        line_coverage: coverage_pct_from_str(&line_coverage_str),
        branch_coverage: coverage_pct_from_str(&branch_coverage_str),
    }
}

fn find_coverage_line(stdout: &String) -> String {
    for line in stdout.lines() {
        if line.contains("TOTAL") {
            return line.to_string();
        }
    }
    panic!(
        "couldn't find coverage percentages in rust-cov output: {}",
        stdout
    );
}

fn coverage_pct_from_str(coverage_str: &str) -> f32 {
    if coverage_str == "-" {
        return 1.0;
    }
    let coverage_str = coverage_str
        .split('%')
        .next()
        .expect(format!("unable to parse coverage percent from: {}", coverage_str).as_str());
    let coverage_pct = coverage_str
        .parse::<f32>()
        .expect("coverage string was not a valid float");
    return coverage_pct / 100.;
}

fn panic_on_fail(msg: &str, output: &Output) {
    if !output.status.success() {
        panic!(
            "{}:\n{}\n{}",
            msg,
            String::from_utf8((&output.stdout).clone()).unwrap(),
            String::from_utf8((&output.stderr).clone()).unwrap()
        );
    }
}
