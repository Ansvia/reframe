use chrono::prelude::*;
use colored::*;
use heck::{CamelCase, KebabCase, MixedCase, ShoutySnakeCase, SnakeCase};
use regex::Regex;
use rustyline::Editor;
use serde_json::Value as JsonValue;

use crate::util;

use std::{
    borrow::Cow,
    collections::HashMap,
    convert::From,
    fmt::Display,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub reframe: ReframeConfig,
    pub project: ProjectConfig,
    pub param: Vec<JsonValue>,
    #[serde(rename = "present", default = "Vec::new")]
    pub presents: Vec<Present>,
    #[serde(default = "Vec::new")]
    pub post_generate: Vec<PostGenerateOp>,
}

#[derive(Debug, Deserialize)]
pub struct Present {
    pub path: String,
    #[serde(rename = "if")]
    pub ifcond: String,
}

#[derive(Debug, Deserialize)]
pub struct PostGenerateOp {
    pub make_executable: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReframeConfig {
    pub name: String,
    pub author: String,
    pub min_version: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "HashMap::new")]
    pub variants: HashMap<String, String>,
    pub version: String,
    pub ignore_dirs: Option<Vec<String>>,
    pub ignore_files: Option<Vec<String>>,
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
struct SkipLine<'a> {
    pub matching: &'a str,
    pub start: bool,
}

impl<'a> SkipLine<'a> {
    #[inline]
    pub fn new() -> Self {
        Self {
            matching: "",
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
    Options,
}

#[derive(Debug, Deserialize, Clone)]
struct Param {
    pub ask: String,
    pub key: String,
    pub default: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "if")]
    pub ifwith: Option<String>,
    #[serde(default = "Vec::new")]
    pub options: Vec<String>,
    // pub autogen: bool,
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
            options: vec![],
            // autogen: false,
            kind: ParamKind::String,
        }
    }
}

pub(crate) struct BuiltinVar {
    pub key: &'static str,
    pub replacer: Box<(dyn Fn(&str) -> String)>,
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
                options: vec![],
                // autogen: true,
                kind: ParamKind::String,
            });
        )*
    };
    ($p:ident, $param:expr, [ $( [$case:expr, $case_func:ident], )* ] ) => {
        make_case_variant!($p, $param, [ $( [$case, $case_func] ),* ])
    }
}

macro_rules! make_case_variants_project {
    ($me:ident, $key:ident, [ $( [$case:expr, $case_func:ident] ),* ] ) => {
        $(
            $me.config.project.variants.insert(
                format!("{}_{}", stringify!($key), $case),
                $me.config.project.$key.$case_func(),
            );
        )*
    };
    ($me:ident, $key:ident, [ $( [$case:expr, $case_func:ident], )* ] ) => {
        make_case_variants_project!($me, $key, [ $( [$case, $case_func] ),* ])
    }
}

const EXCLUDED_EXTS: &[&str] = &[
    "png", "ico", "jpg", "jpeg", "avi", "gif", "mp4", "iso", "zip", "gz", "tar", "rar", "svg",
    "ttf", "woff", "woff2", "eot", "jar", "war", "mpg", "mpeg", "mp3", "m4v", "mkv", "docx",
    "pptx", "pdf", "dmg", "wav", "webm", "m4a", "mov",
];

lazy_static! {
    static ref RE_IF: Regex = Regex::new(r"<% if .*? %>").unwrap();
    static ref ENDIF: &'static str = "<% endif %>";
    static ref RE_SYNTAX_MARK: Regex = Regex::new(r"(#|//|/\*|--)\s*<%(.*)%>").unwrap();
    static ref RE_TEMPLATE_EXT: Regex = Regex::new(r"^(.*)\.template(.\w*)?$").unwrap();
}

pub struct Reframe<'a> {
    pub config: Config,
    param: Vec<Param>,
    builtin_vars: Vec<BuiltinVar>,
    rl: &'a mut Editor<()>,
    path: PathBuf,
    dry_run: bool,
}

impl<'a> Reframe<'a> {
    pub fn open<P: AsRef<Path>>(
        path: P,
        rl: &'a mut Editor<()>,
        dry_run: bool,
    ) -> io::Result<Self> {
        let mut config = read_config(path.as_ref().join("Reframe.toml"))?;

        // check min version
        if util::compare_version(&config.reframe.min_version, env!("CARGO_PKG_VERSION")) < 0 {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Source require min Reframe version {}, please upgrade your Reframe.",
                    config.reframe.min_version
                ),
            ))?;
        }

        if let Some(dirs) = config.project.ignore_dirs.as_mut() {
            dirs.push(".git".to_string());
        }

        let mut builtin_vars = vec![];
        builtin_vars.push(BuiltinVar {
            key: "year",
            replacer: Box::new(|_| Utc::now().format("%Y").to_string()),
        });
        builtin_vars.push(BuiltinVar {
            key: "month_name",
            replacer: Box::new(|_| Utc::now().format("%B").to_string()),
        });

        let param = vec![];
        Ok(Self {
            config,
            param,
            builtin_vars,
            rl,
            path: path.as_ref().to_path_buf(),
            dry_run,
        })
    }

    fn input_read_string(&mut self, ask: String, dflt: String) -> String {
        let rv = self.rl.readline(&ask).unwrap_or_else(|_| dflt.clone());
        if rv.trim().is_empty() {
            dflt
        } else {
            rv.trim().to_owned()
        }
    }

    fn param_value(param: &[Param], k: &str) -> String {
        param
            .iter()
            .find(|a| a.key == k)
            .map(|a| a.value.as_ref().unwrap().to_owned())
            .unwrap_or_else(|| "".to_string())
    }

    #[allow(clippy::option_map_unit_fn)]
    pub fn generate<P: AsRef<Path>>(&mut self, out_dir: P) -> io::Result<String> {
        let out_dir = if self.dry_run {
            Path::new("/tmp").join(out_dir.as_ref())
        } else {
            out_dir.as_ref().to_path_buf()
        };
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

        make_case_variants_project!(
            self,
            name,
            [
                ["lower_case", to_lowercase],
                ["upper_case", to_uppercase],
                ["snake_case", to_snake_case],
                ["kebab_case", to_kebab_case],
                ["camel_case", to_mixed_case],  // eg: variableName
                ["pascal_case", to_camel_case], // eg: ClassName
                ["shout_snake_case", to_shouty_snake_case],
            ]
        );

        let version = self
            .rl
            .readline(&format!(
                "  ➢ {} ({}) : ",
                "Version".bright_blue(),
                &self.config.project.version.yellow()
            ))
            .unwrap_or_else(|_| self.config.project.version.to_owned());

        if version != "" {
            self.config.project.version = version;
        }

        for item in &self.config.param {
            if let JsonValue::Object(o) = &item {
                for (k, item) in o {
                    let ask = get_string(item, "ask", k);
                    let dflt = get_string_option(item, "default");
                    let mut options: Vec<String> = vec![];
                    if let Some(JsonValue::Array(values)) = item.get("options") {
                        options = values
                            .iter()
                            .map(|a| match a {
                                JsonValue::String(a_str) => a_str.to_owned(),
                                _ => panic!("options contains non string value `{}`", &a),
                            })
                            .collect();
                    };

                    let kind = if dflt.as_ref().map(|a| a.as_str()) == Some("true")
                        || dflt.as_ref().map(|a| a.as_str()) == Some("false")
                    {
                        ParamKind::Bool
                    } else if !options.is_empty() {
                        ParamKind::Options
                    } else {
                        ParamKind::String
                    };

                    let p = Param {
                        ask,
                        key: k.clone(),
                        default: dflt,
                        value: None,
                        ifwith: get_string_option(item, "if"),
                        options,
                        // autogen: false,
                        kind,
                    };

                    self.param.push(p);
                }
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
                    if !p.options.is_empty() {
                        format!(
                            "  ➢ {} [{}] ({}) : ",
                            p.ask.bright_blue(),
                            p.options.join("/"),
                            dflt.yellow()
                        )
                    } else {
                        format!("  ➢ {} ({}) : ", p.ask.bright_blue(), dflt.yellow())
                    }
                } else {
                    format!("  ➢ {} : ", p.ask.bright_blue())
                };

                let mut rv = self.rl.readline(&question).map_err(map_err)?;

                rv = rv.trim().to_string();

                if !rv.is_empty() {
                    if !p.options.is_empty() && !p.options.contains(&rv) {
                        println!(
                            "    Value not supported `{}`, only accept {}",
                            rv,
                            p.options.join("/")
                        );
                        continue;
                    }
                } else if p.default.as_ref().is_some() {
                    rv = p.default.as_ref().unwrap().to_owned();
                } else {
                    println!("    Param required: `{}`", &p.key);
                    continue;
                }

                self.rl.add_history_entry(rv.clone());

                p.value = Some(rv);

                break;
            }

            // buat variasi case-nya
            if p.kind == ParamKind::String {
                make_case_variant!(
                    p,
                    self.param,
                    [
                        ["lower_case", to_lowercase],
                        ["upper_case", to_uppercase],
                        ["snake_case", to_snake_case],
                        ["kebab_case", to_kebab_case],
                        ["camel_case", to_mixed_case],
                        ["pascal_case", to_camel_case],
                        ["shout_snake_case", to_shouty_snake_case],
                    ]
                );
            }

            self.param
                .iter_mut()
                .find(|a| a.key == p.key)
                .map(|a| a.value = p.value.to_owned());
        }

        let out_dir = out_dir.join(
            &self
                .config
                .project
                .variants
                .get("name_kebab_case")
                .as_ref()
                .unwrap(),
        );

        // process finish_text
        debug!("processing finish text..");
        self.process_internal_param();

        trace!(
            "copy dir dari `{}` ke `{}`",
            &self.path.display(),
            out_dir.display()
        );

        debug!("remove dir {}", &out_dir.display());
        let _ = fs::remove_dir_all(&out_dir);

        self.copy_dir(self.path.as_path(), out_dir.as_ref())?;

        debug!("processing dir: {}", &out_dir.display());
        self.process_dir(&out_dir)?;

        debug!("normalizing dir: {}", &out_dir.display());
        self.normalize_dirs(&out_dir)?;

        // remove Reframe.toml file
        debug!("remove Reframe.toml ...");
        let path = out_dir.join("Reframe.toml");
        fs::remove_file(&path)?;

        // hapus directory `load.reframe` kalo ada.
        debug!("remove load.reframe dir if any");
        let _ = fs::remove_dir_all(out_dir.join("load.reframe"));

        debug!("Run post_generate procedure...");
        for pg_op in self.config.post_generate.iter() {
            if let Some(path) = pg_op.make_executable.as_ref() {
                let path = Self::string_sub(path, &self.config, &self.param, &self.builtin_vars);
                let path = out_dir.join(path);
                if Path::new(&path).is_file() {
                    if cfg!(unix) {
                        debug!("chmod'ing {}...", path.display());
                        if let Err(e) = Command::new("chmod").arg("+x").arg(&path).output() {
                            error!("Cannot chmod +x `{}`. {}", path.display(), e);
                        }
                    }
                }
            }
        }

        debug!("done.");

        Ok(format!("{}", out_dir.display()))
    }

    /// Memproses parameter internal,
    /// ini harus dijalankan sesudah konfig diproses/parsed.
    #[inline]
    fn process_internal_param(&mut self) {
        if let Some(text) = self.config.project.finish_text.as_ref() {
            self.config.project.finish_text = Some(Self::string_sub(
                text,
                &self.config,
                &self.param,
                &self.builtin_vars,
            ));
        }
    }

    fn copy_dir<P: AsRef<Path>>(&self, src: P, dst: P) -> io::Result<()> {
        let _ = fs::create_dir(&dst);

        let dirent = fs::read_dir(&src)?;

        'dirwalk: for item in dirent {
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
            if self
                .config
                .project
                .ignore_files
                .as_ref()
                .map(|patts| util::file_pattern_match(&tail_name, &patts[..]))
                == Some(true)
            {
                debug!("`{}` ignored", &tail_name.to_string());
                continue;
            }

            // check presents
            for present in &self.config.presents {
                if util::path_to_relative(&path, &self.path).as_path() == Path::new(&present.path) {
                    let mut no_match = 0;
                    for param in &self.param {
                        if param.key == present.ifcond {
                            if param.kind == ParamKind::Bool {
                                if let Some("false") = param.value.as_ref().map(|a| a.as_ref()) {
                                    continue 'dirwalk;
                                }
                            }
                        } else if present.ifcond.contains('=')
                            && present.ifcond.starts_with(&param.key)
                        {
                            if let Some(value) = param.value.as_ref() {
                                if format!("{} == {}", param.key, value) != present.ifcond {
                                    continue 'dirwalk;
                                }
                            } else {
                                // tidak terdefinisikan.
                                continue 'dirwalk;
                            }
                        } else {
                            no_match += 1;
                        }
                    }
                    if no_match == self.param.len() {
                        // param tidak terdefinisikan.
                        continue 'dirwalk;
                    }
                }
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

    fn process_template_str(
        text: String,
        config: &Config,
        param: &[Param],
        builtin_vars: &[BuiltinVar],
    ) -> String {
        let lines: Vec<&str> = text.split('\n').collect();
        let mut new_lines = vec![];
        let mut sl = SkipLine::new();
        let len = lines.len();
        let mut last_if_cond: &str = "";
        let mut last_if_cond_line: usize = 0;

        // proses tahap #1

        for (i, line) in lines.iter().enumerate() {
            if sl.start {
                if line.contains(&sl.matching) {
                    sl.stop();
                }
                if i >= len - 1 {
                    panic!(
                        "unclosed if conditional `{}` at line {}",
                        last_if_cond.trim(),
                        last_if_cond_line
                    );
                }
                continue;
            }

            if RE_IF.is_match(&line) {
                last_if_cond = line;
                last_if_cond_line = i;
                let mut if_handled = false;
                for p in param.iter() {
                    let k = &p.key;
                    let v = match p.value.as_ref() {
                        Some(v) => v,
                        None => {
                            debug!("no value with key: {}", k);
                            continue;
                        }
                    };

                    if k.starts_with("with_") {
                        if v == "false" {
                            if line.contains(&format!("<% if param.{} %>", k)) {
                                debug!("skip...");
                                sl.start = true;
                                sl.matching = "<% endif %>";
                                break;
                            }
                        } else {
                            if_handled = true;
                        }
                    } else {
                        let re_txt = format!(r#"(//|#|--)\s*<% if param.{}\s*==\s*"(.*)" %>"#, k);
                        let re_if_compare = Regex::new(&re_txt).unwrap();
                        let mut caps = re_if_compare.captures_iter(&line);
                        if let Some(cap) = caps.next() {
                            if &cap[2] != v.as_str() {
                                sl.start = true;
                                sl.matching = "<% endif %>";
                            } else {
                                new_lines.push(line.to_owned());
                            }
                            if_handled = true;
                        }
                    }
                }
                // apabila tidak ada param yang menghandle
                // swallow aja semua block-nya.
                if !if_handled {
                    sl.start = true;
                    sl.matching = "<% endif %>";
                } else {
                    new_lines.push(line.to_owned());
                }
            } else if !line.contains(*ENDIF) {
                new_lines.push(line.to_owned());
            }
        }

        // proses tahap #2

        let mut new_lines2 = vec![];

        for line in new_lines {
            if RE_SYNTAX_MARK.is_match(&line) {
                continue;
            }
            new_lines2.push(Self::string_sub(
                line.to_owned(),
                config,
                param,
                builtin_vars,
            ));
        }

        new_lines2.join("\n")
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

        let rv = Self::process_template_str(rv, &self.config, &self.param, &self.builtin_vars);

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

    fn string_sub<'b, S>(
        input: S,
        config: &Config,
        param: &[Param],
        builtin_vars: &[BuiltinVar],
    ) -> String
    where
        S: Into<Cow<'b, str>>,
    {
        let mut rep = input.into().into_owned();
        if !rep.contains('$') {
            return rep;
        }
        rep = rep.replace("$name$", &config.project.name);

        for (k, vr) in config.project.variants.iter() {
            rep = rep.replace(&format!("${}$", k), vr);
        }

        for v in builtin_vars.iter() {
            rep = rep.replace(&format!("${}$", v.key), &(&v.replacer)(&rep));
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

    fn normalize_dirs<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let dirent = fs::read_dir(&path)?;
        for item in dirent {
            let entry = item?;
            let path = entry.path();
            if path.is_dir() {
                let path_str = format!("{}", path.display());
                if RE_TEMPLATE_EXT.is_match(&path_str) {
                    let new_path = RE_TEMPLATE_EXT.replace(&path_str, "$1$2").into_owned();
                    debug!("renaming dir `{}` to `{}`", &path_str, &new_path);
                    fs::rename(&path_str, &new_path)?;
                }
            }
        }
        Ok(())
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
                            &self.builtin_vars,
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

                self.process_template(&path)?;

                // if template file like `README.template.md` then rename to `README.md`.
                // overwrite existing.
                let path_str = format!("{}", path.display());

                if RE_TEMPLATE_EXT.is_match(&path_str) {
                    let new_path = RE_TEMPLATE_EXT.replace(&path_str, "$1$2").into_owned();
                    debug!("renaming `{}` to `{}`", &path_str, &new_path);
                    fs::rename(&path_str, &new_path)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_config(name: &str) -> Config {
        Config {
            reframe: ReframeConfig {
                name: "My Reframe".to_string(),
                author: "robin".to_string(),
                min_version: "0.1.0".to_string(),
            },
            project: ProjectConfig {
                name: name.to_owned(),
                variants: Default::default(),
                version: "0.1.1".to_string(),
                ignore_dirs: None,
                ignore_files: None,
                finish_text: None,
            },
            param: vec![],
            presents: vec![],
            post_generate: vec![],
        }
    }

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
        param.a_pascal = "$param.a_pascal_case$"\
        param.a_shout_snake = "$param.a_shout_snake_case$"\
        "#;

        let expected = r#"\
        project = "Mantap Lah"\
        project_lower = "mantap lah"\
        project_upper = "MANTAP LAH"\

        param.a = "Jumping Fox"\
        param.a_lower = "jumping fox"\
        param.a_snake = "jumping_fox"\
        param.a_camel = "jumpingFox"\
        param.a_pascal = "JumpingFox"\
        param.a_shout_snake = "JUMPING_FOX"\
        "#;

        let name = "Mantap Lah".to_string();

        let config = Config {
            reframe: ReframeConfig {
                name: "My Reframe".to_string(),
                author: "robin".to_string(),
                min_version: "0.1.0".to_string(),
            },
            project: ProjectConfig {
                name: name.to_owned(),
                variants: {
                    let mut h = HashMap::new();
                    h.insert("name".to_string(), name.to_owned());
                    h.insert("name_lower_case".to_string(), name.to_lowercase());
                    h.insert("name_upper_case".to_string(), name.to_uppercase());
                    h.insert("name_kebab_case".to_string(), name.to_kebab_case());
                    h.insert("name_camel_case".to_string(), name.to_mixed_case());
                    h.insert("name_pascal_case".to_string(), name.to_camel_case());
                    h.insert("name_snake_case".to_string(), name.to_snake_case());
                    h
                },
                version: "0.1.1".to_string(),
                ignore_dirs: None,
                ignore_files: None,
                finish_text: None,
            },
            param: vec![],
            presents: vec![],
            post_generate: vec![],
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
                ["camel_case", to_mixed_case],
                ["pascal_case", to_camel_case],
                ["shout_snake_case", to_shouty_snake_case]
            ]
        );

        let output = Reframe::string_sub(input, &config, &param, &[]);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_if_conditional() {
        let input = r#"\
        project = "$name$";
        # <% if param.with_x %>
        import x;
        # <% endif %>
        # <% if param.db == "sqlite" %>
        import sqlite;
        # <% endif %>
        # <% if param.db == "mysql" %>
        import mysql;
        # <% endif %>
        "#;

        let expected1 = r#"\
        project = "Conditional";
        import sqlite;
        "#;
        let expected2 = r#"\
        project = "Conditional";
        import x;
        import sqlite;
        "#;
        let expected3 = r#"\
        project = "Conditional";
        import mysql;
        "#;

        let name = "Conditional";

        let config = build_config(name);

        let p = Param::new("db".to_string(), "sqlite".to_owned());

        let mut param = vec![];
        param.push(p);

        let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
        assert_eq!(output, expected1);

        param.push(Param::new("with_x".to_string(), "true".to_owned()));
        let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
        assert_eq!(output, expected2);

        param.clear();
        param.push(Param::new("with_x".to_string(), "false".to_owned()));
        param.push(Param::new("db".to_string(), "mysql".to_owned()));
        let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
        assert_eq!(output, expected3);
    }

    #[test]
    fn test_if_conditional_comment_mark() {
        let input = r#"\
        project = "$name$";
        -- <% if param.with_x %>
        import x;
        -- <% endif %>
        // <% if param.db == "sqlite" %>
        import sqlite;
        // <% endif %>
        # <% if param.db == "mysql" %>
        import mysql;
        # <% endif %>
        "#;

        let expected1 = r#"\
        project = "Conditional";
        import sqlite;
        "#;

        let name = "Conditional";

        let config = build_config(name);

        let p = Param::new("db".to_string(), "sqlite".to_owned());

        let mut param = vec![];
        param.push(p);
        param.push(Param::new("with_x".to_string(), "false".to_owned()));

        let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
        assert_eq!(output, expected1);
    }

    #[test]
    fn test_if_conditional_sql() {
        let input = r#"\
        -- mulai akun
        -- <% if param.with_account %>
        CREATE TABLE accounts (
            id BIGSERIAL PRIMARY KEY,
            full_name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        );
        -- <% endif %>
        -- selesai
        "#;

        let expected1 = r#"\
        -- mulai akun
        -- selesai
        "#;

        let name = "Conditional";

        let config = build_config(name);

        let mut param = vec![];
        param.push(Param::new("with_account".to_string(), "false".to_owned()));

        let output = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
        assert_eq!(output, expected1);
    }

    #[test]
    #[should_panic(expected = "unclosed if conditional `# <% if param.with_x %>` at line 2")]
    fn test_unclosed_if_tag() {
        let input = r#"\
        project = "$name$";
        # <% if param.with_x %>
        import x;
        import sqlite;
        "#;

        let config = build_config("any");

        let param = vec![];
        let _ = Reframe::process_template_str(input.to_string(), &config, &param, &[]);
    }
}
