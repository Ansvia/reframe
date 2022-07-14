extern crate toml;
#[macro_use]
extern crate serde;
extern crate rustyline;
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate colored;
extern crate env_logger;
extern crate heck;
extern crate regex;
extern crate reqwest;
extern crate zip;
#[macro_use]
extern crate lazy_static;
extern crate chrono;

mod core;
mod util;

use colored::*;
use rustyline::Editor;

use std::{
    env,
    path::{Path, PathBuf},
};

use crate::core::{Param, Reframe};

fn print_usage(args: &[String]) {
    let path = Path::new(&args[0]);
    let exe_name = path.file_name().unwrap().to_str().unwrap();
    println!("Usage: ");
    println!("       ");
    println!("       $ {} [SOURCE] [OPTIONS]", exe_name);
    println!();
    println!("OPTIONS:");
    println!();
    println!("       --dry-run          Test only, don't touch disk.");
    println!("       -P:[key]=[value]   Preset parameters.");
    println!();
    println!("Examples:");
    println!();
    println!("       $ {} anvie/basic-rust", exe_name);
    println!("       $ {} anvie/basic-rust --dry-run", exe_name);
    println!();
}

// extract user's arguments to params
fn extract_params(args: &[String]) -> Vec<Param> {
    args.iter()
        .map(|a| a.trim())
        .filter(|a| a.starts_with("-P"))
        .map(|a| {
            let s = a.split("=").collect::<Vec<&str>>();
            Param::new(s[0].chars().skip(3).collect::<String>(), s[1])
        })
        .collect::<Vec<Param>>()
}

fn main() {
    env_logger::init();

    println!();
    println!(" Reframe {}", env!("CARGO_PKG_VERSION"));
    println!(" project generator tool");
    println!(" by: Robin <r@ansvia.com>");
    println!(" ---------------------------");
    println!();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args[1] == "--help" {
        print_usage(&args);
        return;
    }

    let dry_run = args.contains(&"--dry-run".to_string());

    if dry_run {
        debug!("DRY RUN MODE");
    }

    let source = &args[1];
    let reframe_work_path = env::temp_dir().join("reframe_work");
    let source_path = if !Path::new(&source).exists() {
        debug!("source not found in local: {}", source);
        debug!("trying get from github.com/{} ...", source);
        println!(" downloading from repo...");
        let url = format!(
            "https://github.com/{}.rf/archive/master.zip?nocache={}",
            source,
            util::get_current_time_millis()
        );
        debug!("output: {}", env::temp_dir().display());
        if let Err(e) = util::download(&url, &reframe_work_path, "master.zip") {
            eprintln!(
                "😭 {} {}, while pulling from repo for `{}`",
                "FAILED:".red(),
                e,
                source.bright_blue()
            );
            eprintln!();
            return;
        }
        reframe_work_path.join(format!(
            "{}.rf-master",
            source.split('/').skip(1).collect::<String>()
        ))
    } else {
        PathBuf::from(&source)
    };

    let mut rl = Editor::<()>::new();

    let history_path = env::temp_dir().join(".reframe~");

    if rl.load_history(&history_path).is_err() {
        debug!("no history");
    }

    let params = extract_params(&args);

    let mut rf = match Reframe::open(&source_path, &mut rl, dry_run, params) {
        Ok(rf) => rf,
        Err(e) => {
            eprintln!("{}", format!("{}", e).yellow());
            std::process::exit(2);
        }
    };

    match rf.generate(".") {
        Ok(out_name) => {
            println!();
            println!("  ✨ project generated at `{}`", out_name);
            println!("{}", "     Ready to roll! 😎".green());

            if let Some(text) = rf.config.project.finish_text {
                println!(
                    "________________________________________________\n\n{}",
                    text
                );
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "ERROR".red(), e);
        }
    }
    rl.save_history(&history_path).expect("cannot save history");
}
