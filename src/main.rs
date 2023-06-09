use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use chrono::prelude::*;
use colored::*;
use notify::*;
use notify_debouncer_mini::new_debouncer;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "scriptlink", about = "Watches a folder for file changes.")]
struct Opt {
    #[structopt(short, long, default_value = ".")]
    path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_secs(2), None, tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new(&opt.path), RecursiveMode::Recursive)
        .unwrap();

    println!(
        "{}",
        format!("Watching for changes in {}", &opt.path).white()
    );

    // print all events, non returning
    for events in rx {
        for vec in events {
            for e in vec {
                let _ = process_file(e.path).await;
            }
        }
    }

    Ok(())
}

async fn process_file(path: PathBuf) -> Result<()> {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext == "sh" || ext == "ps1" {


            // Enclose script path in quotes
            let script_path = format!("\"{}\"", path.to_string_lossy());
            println!("{}", format!("Processing file: {}", &script_path).white());
            let script_path = PathBuf::from(script_path);

            // Run the script and get the output.
            let (status, output) = run_script(ext, &script_path)?;

            // Write the output to a file.
            let script_name = path.file_name().unwrap().to_str().unwrap();
            write_output(status, script_name, output)?;
        }
    }

    Ok(())
}

fn run_script<'a>(ext: &'a str, script_path: &'a Path) -> Result<(&'a str, String)> {
    let output = if ext == "sh" {
        Command::new("bash").arg(script_path).output()?
    } else {
        Command::new("powershell")
            .arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(format!("& {}", script_path.to_string_lossy()))
            .output()?
    };

    let (status, output) = if output.status.success() {
        ("OK", String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        ("ERR", String::from_utf8_lossy(&output.stderr).into_owned())
    };

    Ok((status, output))
}

fn write_output(status: &str, script_name: &str, output: String) -> Result<()> {
    println!("{}: {}: {}", status, script_name, output);

    // Create a timestamped filename.
    fs::create_dir_all("results")?;
    let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%S");
    let result_filename = format!("results/{}_{}_{}.txt", script_name, timestamp, status);

    // Write the output to the file.
    let mut result_file = File::create(&result_filename)?;
    result_file.write_all(output.as_bytes())?;

    if status == "OK" {
        println!(
            "{}",
            format!("Results saved in {}", result_filename).white()
        );
    } else {
        println!(
            "{}",
            format!(
                "Error executing script, results saved in {}",
                result_filename
            )
            .red()
        );
    }

    Ok(())
}
