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

mod core;
mod util;

use colored::*;
use rustyline::Editor;

use std::{
    env,
    path::{Path, PathBuf},
};

use crate::core::Reframe;

fn print_usage(args: &Vec<String>) {
    let path = Path::new(args.get(0).unwrap());
    let exe_name = path.file_name().unwrap().to_str().unwrap();
    println!("Usage: ");
    println!("       ");
    println!("       $ {} [SOURCE]", exe_name);
    println!("");
    println!("Example:");
    println!("");
    println!("       $ {} anvie/basic-rust", exe_name);
    println!("");
}

fn main() {
    env_logger::init();

    println!("");
    println!(" Reframe {}", env!("CARGO_PKG_VERSION"));
    println!(" project generator tool");
    println!(" by: Robin <r@ansvia.com>");
    println!(" ---------------------------");
    println!("");

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        print_usage(&args);
        return;
    }

    let source = args.iter().skip(1).next().unwrap();
    let reframe_work_path = env::temp_dir().join("reframe_work");
    let mut source_path = PathBuf::from(&source);

    if !Path::new(&source).exists() {
        debug!("source not found in local: {}", source);
        debug!("trying get from github.com/{} ...", source);
        println!("Downloading from repo...");
        let url = format!("https://github.com/{}.rf/archive/master.zip", source);
        debug!("output: {}", env::temp_dir().display());
        if let Err(e) = util::download(&url, &reframe_work_path) {
            eprintln!(
                "😭 {} {}, while pulling from repo for `{}`",
                "FAILED:".red(),
                e,
                source.bright_blue()
            );
            eprintln!("");
            return;
        }
        source_path = reframe_work_path.join(format!(
            "{}.rf-master",
            source.split("/").skip(1).collect::<String>()
        ));
    }

    let mut rl = Editor::<()>::new();

    if rl.load_history("/tmp/reframe_history~").is_err() {
        debug!("no history");
    }

    let mut rf = Reframe::open(&source_path, rl).expect("Cannot open dir");

    match rf.generate(".") {
        Ok(out_name) => {
            println!("");
            println!("  ✨ project generated at `{}`", out_name);
            println!("{}", "     Ready to roll! 😎".green());

            if let Some(text) = rf.config.project.finish_text {
                println!(
                    "________________________________________________\n\n{}",
                    text
                );
            }
        }
        Err(e) => eprintln!("{}: {}", "ERROR".red(), e),
    }
}
