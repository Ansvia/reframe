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

use colored::*;
use heck::{KebabCase, SnakeCase};
use regex::Regex;
use rustyline::Editor;
use serde_json::Value as JsonValue;
use zip::ZipArchive;

use std::{
    convert::From,
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
struct Config {
    pub reframe: ReframeConfig,
    pub project: ProjectConfig,
    pub param: Vec<JsonValue>,
}

#[derive(Debug, Deserialize)]
struct ReframeConfig {
    pub name: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    pub name: String,
    pub name_snake_case: Option<String>,
    pub version: String,
}

// urus nanti, sementara ini swallow semuanya.
fn map_err<E>(_e: E) -> io::Error {
    io::Error::from(io::ErrorKind::InvalidData)
}

fn read_config<P: AsRef<Path>>(path: P) -> io::Result<Config> {
    let f = fs::read(path)?;
    let rv = String::from_utf8_lossy(f.as_slice());
    toml::from_str(&rv).map_err(map_err)
}

fn get_string(o: &JsonValue, key: &'static str, field: &str) -> String {
    match o.get(key) {
        Some(JsonValue::String(a)) => a.clone(),
        Some(JsonValue::Bool(a)) => format!("{}", a),
        _ => panic!("No `{}` param for `{}`", key, field),
    }
}
fn get_string_option(o: &JsonValue, key: &'static str) -> Option<String> {
    match o.get(key) {
        Some(JsonValue::String(a)) => Some(a.clone()),
        Some(JsonValue::Bool(a)) => Some(format!("{}", a)),
        _ => None,
    }
}

#[derive(Default)]
struct ContUntil {
    pub matching: String,
    pub start: bool,
}

impl ContUntil {
    #[inline]
    pub fn new() -> Self {
        Self {
            matching: Default::default(),
            start: false,
        }
    }
    #[inline]
    pub fn stop(&mut self) {
        self.start = false;
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
enum ParamKind {
    Bool,
    String,
}

#[derive(Debug, Deserialize, Clone)]
struct Param {
    pub ask: String,
    pub key: String,
    pub default: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "if")]
    pub ifwith: Option<String>,
    pub autogen: bool,
    pub kind: ParamKind,
}

macro_rules! make_case_variant {
    ($case:expr, $case_func:ident, $param:expr, $p:ident) => {
        $param.push(Param {
            ask: Default::default(),
            key: format!("{}_{}", $p.key, $case),
            default: $p.default.as_ref().map(|a| a.$case_func().to_owned()),
            value: $p.value.as_ref().map(|a| a.$case_func().to_owned()),
            ifwith: $p.ifwith.clone(),
            autogen: true,
            kind: ParamKind::String,
        });
    };
}

struct Reframe {
    config: Config,
    param: Vec<Param>,
    rl: Editor<()>,
    path: PathBuf,
}

impl Reframe {
    pub fn open<P: AsRef<Path>>(path: P, rl: Editor<()>) -> io::Result<Self> {
        let config = read_config(path.as_ref().join("Reframe.toml"))?;
        let param = vec![];
        Ok(Self {
            config,
            param,
            rl,
            path: path.as_ref().to_path_buf(),
        })
    }

    fn input_read_string(&mut self, ask: String, dflt: String) -> String {
        let rv = self.rl.readline(&ask).unwrap_or_else(|_| dflt.clone());
        if rv.trim().len() == 0 {
            dflt
        } else {
            rv.trim().to_owned()
        }
    }

    fn param_value(param: &Vec<Param>, k: &str) -> String {
        param
            .iter()
            .find(|a| a.key == k)
            .map(|a| a.value.as_ref().unwrap().to_owned())
            .unwrap_or("".to_string())
    }

    pub fn generate<P: AsRef<Path>>(&mut self, out_dir: P) -> io::Result<String> {
        let project_name = self.input_read_string(
            format!(
                "  ➢ {} ({}) : ",
                "Project name".bright_blue(),
                &self.config.project.name
            ),
            self.config.project.name.clone(),
        );

        if project_name != "" {
            self.config.project.name = project_name;
        }

        self.config.project.name_snake_case = Some(self.config.project.name.to_kebab_case());

        let version = self
            .rl
            .readline(&format!(
                "  ➢ {} ({}) : ",
                "Version".bright_blue(),
                &self.config.project.version
            ))
            .unwrap_or(self.config.project.version.clone());

        if version != "" {
            self.config.project.version = version;
        }

        for item in &self.config.param {
            match &item {
                JsonValue::Object(o) => {
                    for (k, item) in o {
                        let ask = get_string(item, "ask", k);
                        let dflt = get_string_option(item, "default");

                        let kind = if dflt == Some("true".to_string())
                            || dflt == Some("false".to_string())
                        {
                            ParamKind::Bool
                        } else {
                            ParamKind::String
                        };

                        let p = Param {
                            ask: ask,
                            key: k.clone(),
                            default: dflt,
                            value: None,
                            ifwith: get_string_option(item, "if"),
                            autogen: false,
                            kind,
                        };

                        self.param.push(p);
                    }
                }
                _ => (),
            }
        }

        let mut new_param = self.param.clone();

        for p in new_param.iter_mut() {
            if let Some(depends) = p.ifwith.as_ref() {
                if Self::param_value(&self.param, depends) == "false" {
                    continue;
                }
            }

            loop {
                let question = if let Some(dflt) = p.default.as_ref() {
                    format!("  ➢ {} ({}) : ", p.ask.bright_blue(), dflt.yellow())
                } else {
                    format!("  ➢ {} : ", p.ask.bright_blue())
                };

                let rv = self.rl.readline(&question).map_err(map_err)?;
                let rv = if rv.trim().len() > 0 {
                    Some(&rv)
                } else {
                    if p.default.as_ref().is_some() {
                        p.default.as_ref()
                    } else {
                        println!("Parameter harus diisi: `{}`", &p.key);
                        continue;
                    }
                };
                p.value = rv.map(|a| a.to_owned());
                break;
            }

            // buat variasi case-nya
            if p.kind == ParamKind::String {
                make_case_variant!("lowercase", to_lowercase, self.param, p);
                make_case_variant!("snake_case", to_snake_case, self.param, p);
                make_case_variant!("kebab_case", to_kebab_case, self.param, p);
            }

            self.param
                .iter_mut()
                .find(|a| a.key == p.key)
                .map(|a| a.value = p.value.clone());
        }

        let out_dir = out_dir
            .as_ref()
            .join(self.config.project.name_snake_case.as_ref().unwrap());

        trace!(
            "copy dir dari `{}` ke `{}`",
            &self.path.display(),
            out_dir.display()
        );

        let _ = fs::remove_dir_all(&out_dir);

        self.copy_dir(self.path.as_path(), out_dir.as_ref())?;

        debug!("processing dir: {}", &out_dir.display());
        self.process_dir(&out_dir)?;

        // hapus file Reframe.toml
        let path = out_dir.join("Reframe.toml");
        fs::remove_file(&path)?;

        // hapus directory `load.reframe`
        fs::remove_dir_all(out_dir.join("load.reframe"))?;

        Ok(format!("{}", out_dir.display()))
    }

    fn copy_dir<P: AsRef<Path>>(&self, src: P, dst: P) -> io::Result<()> {
        let _ = fs::create_dir(&dst);

        let dirent = fs::read_dir(&src)?;

        for item in dirent {
            let entry = item?;
            let path = entry.path();
            trace!("path: {}", &path.display());
            if path.is_dir() {
                debug!("visit: {}", &path.display());
                let dst = dst
                    .as_ref()
                    .join(path.file_name().unwrap().to_str().unwrap());
                fs::create_dir_all(&dst)?;
                self.copy_dir(&path, &dst)?;
            } else {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                let dst = dst.as_ref().join(file_name);
                trace!("copy: {} -> {}", &path.display(), &dst.display());
                fs::copy(&path, &dst)?;
            }
        }

        Ok(())
    }

    fn process_template<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        debug!("processing template: {}", path.as_ref().display());

        let mut rv: String = String::from_utf8_lossy(fs::read(&path)?.as_slice()).to_string();
        rv = rv.replace(
            "$name_snake_case$",
            self.config.project.name_snake_case.as_ref().unwrap(),
        );

        let lines = rv.split('\n');
        let mut new_lines = vec![];
        let mut continue_until = ContUntil::new();
        let mut skip_counter = 0;

        // proses tahap #1

        let re_if = Regex::new(r"<% if param\.(\S+) %>").unwrap();
        let re_ignore = Regex::new(r"<% endif %>").unwrap();

        for line in lines.clone() {
            if continue_until.start {
                if line.contains(&continue_until.matching) {
                    continue_until.stop();
                }
                continue;
            }

            if re_if.is_match(&line) {
                for p in self.param.iter() {
                    let k = &p.key;
                    let v = p.value.as_ref().unwrap();
                    if skip_counter > 0 {
                        skip_counter -= 1;
                        continue;
                    }
                    if k.starts_with("with_") {
                        if v == "false" {
                            if line.contains(&format!("<% if param.{} %>", k)) {
                                debug!("skip...");
                                continue_until.start = true;
                                continue_until.matching = "<% endif %>".to_string();
                                break;
                            }
                        } else {
                            skip_counter = 1;
                        }
                    }
                }
            } else {
                if !re_ignore.is_match(&line) {
                    new_lines.push(line.to_string());
                }
            }
        }

        // proses tahap #2

        let re_ignore2 = Regex::new(r"<%(.*)%>").unwrap();

        let mut new_lines2 = vec![];

        for line in new_lines {
            if re_ignore2.is_match(&line) {
                continue;
            }
            let mut x = line.to_owned();
            x = x.replace("$name$", &self.config.project.name);
            x = x.replace(
                "$name_snake_case$",
                self.config.project.name_snake_case.as_ref().unwrap(),
            );
            x = x.replace("$version$", &self.config.project.version);
            for p in self.param.iter() {
                if let Some(value) = p.value.as_ref() {
                    let to_rep = format!("$param.{}$", p.key);
                    trace!("replacing `{}` -> `{}`", to_rep, value);
                    x = x.replace(&to_rep, value);
                }
            }
            new_lines2.push(x.clone());
        }

        let rv = new_lines2.join("\n");

        let out_path = format!("{}", path.as_ref().display());

        let _ = fs::remove_file(&out_path);

        let mut fout = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&out_path)
            .unwrap_or_else(|_| panic!("cannot open out path: {}", out_path));

        writeln!(fout, "{}", rv)?;

        Ok(())
    }

    fn process_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let dirent = fs::read_dir(path)?;

        for item in dirent {
            let entry = item?;
            let path = entry.path();
            if path.is_dir() {
                self.process_dir(&path)?;
            } else {
                self.process_template(path)?;
            }
        }

        Ok(())
    }
}

fn extract<P: AsRef<Path>>(zip_path: P, out_dir: P) -> io::Result<()> {
    let fin = File::open(&zip_path)
        .unwrap_or_else(|_| panic!("Cannot open zip file `{}`", zip_path.as_ref().display()));
    let mut archive = ZipArchive::new(fin)?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .unwrap_or_else(|e| panic!("Cannot get zip entry index `{}`: {}", i, e));
        let outpath = out_dir.as_ref().join(file.sanitized_name());

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (&*file.name()).ends_with('/') {
            debug!(
                "File {} extracted to \"{}\"",
                i,
                outpath.as_path().display()
            );
            fs::create_dir_all(&outpath)?;
        } else {
            debug!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.as_path().display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}

fn download<P: AsRef<Path>>(url: &str, out_dir: P) -> io::Result<()> {
    fs::create_dir_all(&out_dir)?;

    let out_path = out_dir.as_ref().join("master.zip");

    {
        let mut w = File::create(&out_path)?;

        debug!("downloading {} ...", url);

        reqwest::get(url)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("cannot clone from github: {} (error: {})", url, e),
                )
            })?
            .copy_to(&mut w)
            .unwrap_or_else(|_| panic!("cannot store data to `{}`", out_path.display()));

        println!("");
    }

    // extract zip file
    let zip_file = out_path.clone();

    debug!(
        "extracting `{}` to `{}` ...",
        &zip_file.display(),
        &out_dir.as_ref().display()
    );

    if extract(zip_file, out_dir.as_ref().to_path_buf()).is_err() {
        Err(io::Error::from(io::ErrorKind::NotFound))?;
    }

    Ok(())
}

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
        if let Err(e) = download(&url, &reframe_work_path) {
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
            println!("");
        }
        Err(e) => eprintln!("{}", e),
    }
}
