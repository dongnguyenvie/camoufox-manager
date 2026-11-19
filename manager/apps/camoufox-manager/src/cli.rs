//! CLI (phase 12): an agent surface over the running app's HTTP API (`http://127.0.0.1:$CM_PORT`).
//!
//! - `spec.rs`: every command declared as data; generates `--help`, `docs`, `skill`.
//! - `commands.rs`: HTTP client + one handler per command.
//! - This file: argument parsing against the specs, output, exit codes.
//!
//! Contract: success prints the result on stdout (JSON with `--json`, text otherwise) and exits 0;
//! failure prints one line `error: <code>: <message>` on stderr and exits 1.

mod commands;
mod spec;

use anyhow::Result;
use serde_json::Value;
use spec::{COMMANDS, Command, GLOBAL_FLAGS};
use std::collections::HashMap;

/// Failure reported as `error: <code>: <message>`.
#[derive(Debug)]
pub struct CliError {
    pub code: String,
    pub message: String,
}

impl CliError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }

    fn usage(message: impl Into<String>) -> Self {
        Self::new("usage", message)
    }
}

/// What a command produced: the JSON for `--json`, the text otherwise.
pub struct Output {
    pub json: Value,
    pub text: String,
}

/// Parsed command line after the program name.
#[derive(Debug, Default)]
pub struct Parsed {
    pub words: Vec<String>,
    /// flag name → values (switches have one empty value).
    pub flags: HashMap<String, Vec<String>>,
}

impl Parsed {
    pub fn has(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }

    pub fn one(&self, name: &str) -> Option<&str> {
        self.flags.get(name).and_then(|v| v.last()).map(String::as_str)
    }

    /// All values, comma-separated entries split.
    pub fn list(&self, name: &str) -> Vec<String> {
        self.flags
            .get(name)
            .into_iter()
            .flatten()
            .flat_map(|v| v.split(','))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

fn takes_value(name: &str) -> bool {
    GLOBAL_FLAGS.iter().chain(COMMANDS.iter().flat_map(|c| c.flags.iter())).any(|f| f.name == name && f.value.is_some())
}

fn parse(args: &[String]) -> Result<Parsed, CliError> {
    let mut p = Parsed::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "-h" {
            p.flags.entry("help".into()).or_default().push(String::new());
        } else if let Some(flag) = a.strip_prefix("--") {
            let (name, inline) = match flag.split_once('=') {
                Some((n, v)) => (n, Some(v.to_string())),
                None => (flag, None),
            };
            let value = if takes_value(name) {
                match inline {
                    Some(v) => v,
                    None => it.next().cloned().ok_or_else(|| CliError::usage(format!("--{name} needs a value")))?,
                }
            } else if inline.is_some() {
                return Err(CliError::usage(format!("--{name} does not take a value")));
            } else {
                String::new()
            };
            p.flags.entry(name.to_string()).or_default().push(value);
        } else {
            p.words.push(a.clone());
        }
    }
    Ok(p)
}

/// Resolve the command and check flags / positional count against its spec.
fn resolve(p: &Parsed) -> Result<(&'static Command, Vec<&str>), CliError> {
    let words: Vec<&str> = p.words.iter().map(String::as_str).collect();
    let cmd = spec::find(&words).ok_or_else(|| {
        let known = COMMANDS.iter().map(|c| c.name).collect::<Vec<_>>().join(", ");
        CliError::usage(format!("unknown command `{}` (commands: {known})", words.join(" ")))
    })?;
    let rest = words[cmd.name.split(' ').count()..].to_vec();
    for name in p.flags.keys() {
        if !GLOBAL_FLAGS.iter().chain(cmd.flags).any(|f| f.name == name) {
            return Err(CliError::usage(format!("unknown flag --{name} for `{}`", cmd.name)));
        }
    }
    for f in cmd.flags.iter().filter(|f| f.required) {
        if !p.has(f.name) {
            return Err(CliError::usage(format!("`{}` needs --{} (usage: {})", cmd.name, f.name, spec::usage(cmd))));
        }
    }
    for f in cmd.flags.iter().filter(|f| !f.repeatable) {
        if p.flags.get(f.name).is_some_and(|v| v.len() > 1) {
            return Err(CliError::usage(format!("--{} given more than once", f.name)));
        }
    }
    if rest.len() != cmd.args.len() {
        return Err(CliError::usage(format!("`{}` takes {} argument(s) (usage: {})", cmd.name, cmd.args.len(), spec::usage(cmd))));
    }
    Ok((cmd, rest))
}

async fn execute(p: &Parsed) -> Result<Option<Output>, CliError> {
    if p.words.is_empty() || (p.words[0] == "help" && p.words.len() == 1) {
        print!("{}", spec::top_help());
        return Ok(None);
    }
    if p.words[0] == "help" {
        // `help profile start` == `profile start --help`
        let words: Vec<&str> = p.words[1..].iter().map(String::as_str).collect();
        let cmd = spec::find(&words).ok_or_else(|| CliError::usage(format!("unknown command `{}`", words.join(" "))))?;
        print!("{}", spec::command_help(cmd));
        return Ok(None);
    }
    if p.words[0] == "version" {
        println!("camoufox-manager {}", env!("CARGO_PKG_VERSION"));
        return Ok(None);
    }
    let (cmd, args) = match resolve(p) {
        Ok(r) => r,
        // `--help` wins over argument errors (`profile start --help` has no id).
        Err(e) if p.has("help") => {
            let words: Vec<&str> = p.words.iter().map(String::as_str).collect();
            match spec::find(&words) {
                Some(c) => {
                    print!("{}", spec::command_help(c));
                    return Ok(None);
                }
                None => return Err(e),
            }
        }
        Err(e) => return Err(e),
    };
    if p.has("help") {
        print!("{}", spec::command_help(cmd));
        return Ok(None);
    }
    match cmd.name {
        "skill" => {
            print!("{}", spec::skill());
            return Ok(None);
        }
        "docs" => {
            print!("{}", spec::markdown());
            return Ok(None);
        }
        _ => {}
    }
    commands::run(cmd, &args, p).await.map(Some)
}

pub fn run(args: Vec<String>) -> Result<()> {
    let outcome = match parse(&args) {
        Ok(p) => {
            let json = p.has("json");
            tokio::runtime::Runtime::new()?.block_on(execute(&p)).map(|o| (json, o))
        }
        Err(e) => Err(e),
    };
    match outcome {
        Ok((json, Some(out))) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&out.json)?);
            } else {
                println!("{}", out.text.trim_end());
            }
            Ok(())
        }
        Ok((_, None)) => Ok(()),
        Err(e) => {
            eprintln!("error: {}: {}", e.code, e.message.replace('\n', " "));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Parsed, CliError> {
        parse(&s.split_whitespace().map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn parses_flags_and_words() {
        let a = p("profile create --name=Shop --os mac --json").unwrap();
        assert_eq!(a.words, ["profile", "create"]);
        assert_eq!(a.one("name"), Some("Shop"));
        assert_eq!(a.one("os"), Some("mac"));
        assert!(a.has("json"));
        let (cmd, rest) = resolve(&a).unwrap();
        assert_eq!(cmd.name, "profile create");
        assert!(rest.is_empty());
    }

    #[test]
    fn repeatable_and_lists() {
        let a = p("proxy assign 2 --profile a,b --profile c").unwrap();
        assert_eq!(a.list("profile"), ["a", "b", "c"]);
        assert_eq!(resolve(&a).unwrap().1, ["2"]);
    }

    #[test]
    fn rejects_bad_usage() {
        assert!(resolve(&p("profile start").unwrap()).is_err(), "missing id");
        assert!(resolve(&p("profile list x").unwrap()).is_err(), "extra arg");
        assert!(resolve(&p("profile create").unwrap()).is_err(), "missing --name");
        assert!(resolve(&p("status --bogus").unwrap()).is_err());
        assert!(resolve(&p("nope").unwrap()).is_err());
        assert!(p("status --json=1").is_err());
        assert!(p("profile create --name").is_err());
    }
}
