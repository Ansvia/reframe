use colored::*;
use heck::{CamelCase, KebabCase, ShoutySnakeCase, SnakeCase};
use regex::Regex;
use rustyline::Editor;
use serde_json::Value as JsonValue;

use std::{
    borrow::Cow,
    collections::HashMap,
    convert::From,
    fmt::Display,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub reframe: ReframeConfig,
    pub project: ProjectConfig,
    pub param: Vec<JsonValue>,
}

#[derive(Debug, Deserialize)]
pub struct ReframeConfig {
    pub name: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "HashMap::new")]
    pub variants: HashMap<String, String>,
    pub version: String,
    pub ignore_dirs: Option<Vec<String>>,
    pub finish_text: Option<String>,
}

fn map_err<E: Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, format!("{}", e))
}

fn read_config<P: AsRef<Path>>(path: P) -> io::Result<Config> {
    let f = fs::read(path)?;
    let rv = String::from_utf8_lossy(f.as_slice());
    toml::from_str(&rv).map_err(map_err)
}

fn get_string(o: &JsonValue, key: &'static str, field: &str) -> String {
    match o.get(key) {
        Some(JsonValue::String(a)) => a.to_owned(),
        Some(JsonValue::Bool(a)) => format!("{}", a),
        _ => panic!("No `{}` param for `{}`", key, field),
    }
}
fn get_string_option(o: &JsonValue, key: &'static str) -> Option<String> {
    match o.get(key) {
        Some(JsonValue::String(a)) => Some(a.to_owned()),
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

impl Param {
    #[cfg(test)]
    pub fn new(key: String, value: String) -> Self {
        Param {
            ask: Default::default(),
            key,
            default: Default::default(),
            value: Some(value),
            ifwith: None,
            autogen: false,
            kind: ParamKind::String,
        }
    }
}

macro_rules! make_case_variant {
    ($p:ident, $param:expr, [ $( [$case:expr, $case_func:ident] ),* ] ) => {
        $(
            $param.push(Param {
                ask: Default::default(),
                key: format!("{}_{}", $p.key, $case),
                default: $p.default.as_ref().map(|a| a.$case_func().to_owned()),
                value: $p.value.as_ref().map(|a| a.$case_func().to_owned()),
                ifwith: $p.ifwith.clone(),
                autogen: true,
                kind: ParamKind::String,
            });
        )*
    };
}

const EXCLUDED_EXTS: &'static [&'static str] = &[
    "png", "ico", "jpg", "jpeg", "avi", "gif", "mp4", "iso", "zip", "gz", "tar", "rar", "svg",
    "ttf", "woff", "woff2", "eot",
];

lazy_static! {
    static ref RE_IF: Regex = Regex::new(r"<% if param\.(\S+) %>").unwrap();
    static ref RE_ENDIF: Regex = Regex::new(r"<% endif %>").unwrap();
    static ref RE_SYNTAX_MARK: Regex = Regex::new(r"(#|//|/\*)\s*<%(.*)%>").unwrap();
}

pub struct Reframe {
    pub config: Config,
    param: Vec<Param>,
    rl: Editor<()>,
    path: PathBuf,
}

impl Reframe {
    pub fn open<P: AsRef<Path>>(path: P, rl: Editor<()>) -> io::Result<Self> {
        let mut config = read_config(path.as_ref().join("Reframe.toml"))?;

        match config.project.ignore_dirs.as_mut() {
            Some(dirs) => {
                dirs.push(".git".to_string());
            }
            None => (),
        }

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
                &self.config.project.name.yellow()
            ),
            self.config.project.name.to_owned(),
        );

        if project_name != "" {
            self.config.project.name = project_name;
        }

        self.config.project.variants.insert(
            "name_lower_case".to_string(),
            self.config.project.name.to_lowercase(),
        );
        self.config.project.variants.insert(
            "name_upper_case".to_string(),
            self.config.project.name.to_uppercase(),
        );
        self.config.project.variants.insert(
            "name_snake_case".to_string(),
            self.config.project.name.to_snake_case(),
        );
        self.config.project.variants.insert(
            "name_kebab_case".to_string(),
            self.config.project.name.to_kebab_case(),
        );
        self.config.project.variants.insert(
            "name_camel_case".to_string(),
            self.config.project.name.to_camel_case(),
        );
        self.config.project.variants.insert(
            "name_shout_snake_case".to_string(),
            self.config.project.name.to_shouty_snake_case(),
        );

        let version = self
            .rl
            .readline(&format!(
                "  ➢ {} ({}) : ",
                "Version".bright_blue(),
                &self.config.project.version.yellow()
            ))
            .unwrap_or(self.config.project.version.to_owned());

        if version != "" {
            self.config.project.version = version;
        }

        for item in &self.config.param {
            match &item {
                JsonValue::Object(o) => {
                    for (k, item) in o {
                        let ask = get_string(item, "ask", k);
                        let dflt = get_string_option(item, "default");

                        let kind = if dflt.as_ref().map(|a| a.as_str()) == Some("true")
                            || dflt.as_ref().map(|a| a.as_str()) == Some("false")
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
                make_case_variant!(
                    p,
                    self.param,
                    [
                        // @TODO(robin): remove this `lowercase` (backward compatibility code).
                        ["lowercase", to_lowercase],
                        ["lower_case", to_lowercase],
                        ["upper_case", to_uppercase],
                        ["snake_case", to_snake_case],
                        ["kebab_case", to_kebab_case],
                        ["camel_case", to_camel_case],
                        ["shout_snake_case", to_shouty_snake_case]
                    ]
                );
            }

            self.param
                .iter_mut()
                .find(|a| a.key == p.key)
                .map(|a| a.value = p.value.to_owned());
        }

        let out_dir = out_dir.as_ref().join(
            &self
                .config
                .project
                .variants
                .get("name_kebab_case")
                .as_ref()
                .unwrap(),
        );

        // process finish_text
        self.process_internal_param();

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

        // hapus directory `load.reframe` kalo ada.
        let _ = fs::remove_dir_all(out_dir.join("load.reframe"));

        Ok(format!("{}", out_dir.display()))
    }

    /// Memproses parameter internal,
    /// ini harus dijalankan sesudah konfig diproses/parsed.
    fn process_internal_param(&mut self) {
        if let Some(text) = self.config.project.finish_text.as_ref() {
            self.config.project.finish_text =
                Some(Self::string_sub(text, &self.config, &self.param));
        }
    }

    fn copy_dir<P: AsRef<Path>>(&self, src: P, dst: P) -> io::Result<()> {
        let _ = fs::create_dir(&dst);

        let dirent = fs::read_dir(&src)?;

        for item in dirent {
            let entry = item?;
            let path = entry.path();
            trace!("path: {}", &path.display());
            let tail_name = path.file_name().unwrap().to_str().unwrap();
            if self
                .config
                .project
                .ignore_dirs
                .as_ref()
                .map(|dirs| dirs.contains(&tail_name.to_string()))
                == Some(true)
            {
                debug!("`{}` ignored", &path.display());
                continue;
            }
            if path.is_dir() {
                debug!("visit: {}", &path.display());
                let dst = dst.as_ref().join(tail_name);
                fs::create_dir_all(&dst)?;
                self.copy_dir(&path, &dst)?;
            } else {
                let file_name = tail_name;
                let dst = dst.as_ref().join(file_name);
                trace!("copy: {} -> {}", &path.display(), &dst.display());
                fs::copy(&path, &dst)?;
            }
        }

        Ok(())
    }

    fn process_template_str(text: String, param: &Vec<Param>, config: &Config) -> String {
        let lines = text.split('\n');
        let mut new_lines = vec![];
        let mut continue_until = ContUntil::new();
        let mut skip_counter = 0;

        // proses tahap #1

        for line in lines.clone() {
            if continue_until.start {
                if line.contains(&continue_until.matching) {
                    continue_until.stop();
                }
                continue;
            }

            if RE_IF.is_match(&line) {
                for p in param.iter() {
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
                if !RE_ENDIF.is_match(&line) {
                    new_lines.push(line.to_string());
                }
            }
        }

        // proses tahap #2

        let mut new_lines2 = vec![];

        for line in new_lines {
            if RE_SYNTAX_MARK.is_match(&line) {
                continue;
            }
            new_lines2.push(Self::string_sub(line.to_owned(), config, param));
        }

        let rv = new_lines2.join("\n");

        rv
    }

    fn process_template<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        debug!("processing template: {}", path.as_ref().display());
        print!(".");
        io::stdout().flush().unwrap();

        let rv: String = String::from_utf8_lossy(
            fs::read(&path)
                .unwrap_or_else(|_| panic!("cannot read: {}", path.as_ref().display()))
                .as_slice(),
        )
        .to_string();

        let rv = Self::process_template_str(rv, &self.param, &self.config);

        let out_path = format!("{}", path.as_ref().display());

        let _ = fs::remove_file(&out_path);

        let mut fout = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&out_path)
            .unwrap_or_else(|_| panic!("cannot open out path: {}", out_path));

        writeln!(fout, "{}", rv).unwrap_or_else(|_| panic!("cannot write `{}`", out_path));

        Ok(())
    }

    fn string_sub<'a, S>(input: S, config: &Config, param: &Vec<Param>) -> String
    where
        S: Into<Cow<'a, str>>,
    {
        let mut rep = input.into().into_owned();
        if !rep.contains('$') {
            return rep;
        }
        rep = rep.replace("$name$", &config.project.name);

        for (k, vr) in config.project.variants.iter() {
            rep = rep.replace(&format!("${}$", k), vr);
        }

        rep = rep.replace("$version$", &config.project.version);
        for p in param.iter() {
            if let Some(value) = p.value.as_ref() {
                let to_rep = format!("$param.{}$", p.key);
                rep = rep.replace(&to_rep, value);
            }
        }
        rep
    }

    fn process_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let dirent = fs::read_dir(&path)
            .unwrap_or_else(|_| panic!("cannot read dir `{}`", path.as_ref().display()));

        for item in dirent {
            let entry = item?;
            let mut path = entry.path();

            let pbs = PathBuf::from(
                path.to_path_buf()
                    .iter()
                    .map(|pb| {
                        Self::string_sub(
                            format!("{}", pb.to_string_lossy()),
                            &self.config,
                            &self.param,
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("/"),
            );

            if path != pbs {
                // ganti path ke terbaru yang telah update
                // templating untuk path-nya.
                fs::rename(&path, &pbs)?;
                path = pbs;
            }

            if path.is_dir() {
                self.process_dir(&path)?;
            } else {
                if let Some(ext) = path.extension() {
                    if EXCLUDED_EXTS.contains(&ext.to_str().unwrap()) {
                        debug!("ignored: {}", path.display());
                        continue;
                    }
                }
                self.process_template(path)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_sub() {
        let input = r#"\
        project = "$name$"\
        project_lower = "$name_lower_case$"\
        project_upper = "$name_upper_case$"\

        param.a = "$param.a$"\
        param.a_lower = "$param.a_lower_case$"\
        param.a_snake = "$param.a_snake_case$"\
        param.a_camel = "$param.a_camel_case$"\
        param.a_shout_snake = "$param.a_shout_snake_case$"\
        "#;

        let expected = r#"\
        project = "Mantap Lah"\
        project_lower = "mantap lah"\
        project_upper = "MANTAP LAH"\

        param.a = "Jumping Fox"\
        param.a_lower = "jumping fox"\
        param.a_snake = "jumping_fox"\
        param.a_camel = "JumpingFox"\
        param.a_shout_snake = "JUMPING_FOX"\
        "#;

        let name = "Mantap Lah".to_string();

        let config = Config {
            reframe: ReframeConfig {
                name: "My Reframe".to_string(),
                author: "robin".to_string(),
            },
            project: ProjectConfig {
                name: name.to_owned(),
                variants: {
                    let mut h = HashMap::new();
                    h.insert("name".to_string(), name.to_owned());
                    h.insert("name_lower_case".to_string(), name.to_lowercase());
                    h.insert("name_upper_case".to_string(), name.to_uppercase());
                    h.insert("name_kebab_case".to_string(), name.to_kebab_case());
                    h.insert("name_camel_case".to_string(), name.to_camel_case());
                    h.insert("name_snake_case".to_string(), name.to_snake_case());
                    h
                },
                version: "0.1.1".to_string(),
                ignore_dirs: None,
                finish_text: None,
            },
            param: vec![],
        };

        let p = Param::new("a".to_string(), "Jumping Fox".to_string());

        let mut param = vec![];
        param.push(p.clone());
        make_case_variant!(
            p,
            param,
            [
                ["lower_case", to_lowercase],
                ["upper_case", to_uppercase],
                ["snake_case", to_snake_case],
                ["camel_case", to_camel_case],
                ["shout_snake_case", to_shouty_snake_case]
            ]
        );

        let output = Reframe::string_sub(input, &config, &param);
        assert_eq!(output, expected);
    }
}
