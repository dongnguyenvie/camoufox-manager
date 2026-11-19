//! The CLI declared once, as data.
//!
//! [`COMMANDS`] is the single source for argument parsing (`cli.rs`), `--help`, the markdown
//! reference (`camoufox-manager docs`) and the agent skill file (`camoufox-manager skill`), so the
//! three can never drift apart. The usage line is derived from the args and flags.

pub struct Arg {
    pub name: &'static str,
    pub summary: &'static str,
}

pub struct Flag {
    pub name: &'static str,
    /// Placeholder of the value (`<name>`); `None` = boolean switch.
    pub value: Option<&'static str>,
    pub summary: &'static str,
    pub required: bool,
    /// May be given several times and/or as a comma-separated list.
    pub repeatable: bool,
}

pub struct Command {
    /// One or two words: `status`, `profile list`.
    pub name: &'static str,
    pub summary: &'static str,
    /// Required positional arguments, in order.
    pub args: &'static [Arg],
    pub flags: &'static [Flag],
    pub examples: &'static [&'static str],
    /// Needs the running app (everything but `help`, `docs`, `skill`, `version`).
    pub needs_app: bool,
}

const fn flag(name: &'static str, value: &'static str, summary: &'static str) -> Flag {
    Flag { name, value: Some(value), summary, required: false, repeatable: false }
}

const fn required(name: &'static str, value: &'static str, summary: &'static str) -> Flag {
    Flag { name, value: Some(value), summary, required: true, repeatable: false }
}

const ID: Arg = Arg { name: "id", summary: "profile id (see `profile list`)" };

/// Flags every command accepts.
pub const GLOBAL_FLAGS: &[Flag] = &[
    Flag { name: "json", value: None, summary: "print machine-readable JSON on stdout", required: false, repeatable: false },
    flag("api-key", "<key>", "API key sent as X-API-Key (default: $CM_API_KEY)"),
    flag("port", "<n>", "port of the running app (default: $CM_PORT, else 7878)"),
    Flag { name: "help", value: None, summary: "show help for the command", required: false, repeatable: false },
];

pub const COMMANDS: &[Command] = &[
    Command {
        name: "status",
        summary: "Show whether the app is reachable, the Camoufox engine state and profile counts",
        args: &[],
        flags: &[],
        examples: &["camoufox-manager status", "camoufox-manager status --json"],
        needs_app: true,
    },
    Command {
        name: "profile list",
        summary: "List profiles with their OS, proxy and running state",
        args: &[],
        flags: &[],
        examples: &["camoufox-manager profile list --json"],
        needs_app: true,
    },
    Command {
        name: "profile create",
        summary: "Create a profile with a new pinned machine (needs Camoufox installed)",
        args: &[],
        flags: &[
            required("name", "<name>", "profile name"),
            flag("os", "<win|mac|lin>", "operating system of the pinned machine (default win)"),
        ],
        examples: &["camoufox-manager profile create --name \"Shop EU\" --os mac --json"],
        needs_app: true,
    },
    Command {
        name: "profile show",
        summary: "Show one profile: settings, pinned machine summary, proxy, running state",
        args: &[ID],
        flags: &[],
        examples: &["camoufox-manager profile show 3f9c2a7d1e8b4c5a6d7e8f90 --json"],
        needs_app: true,
    },
    Command {
        name: "profile start",
        summary: "Start a profile's browser; with --json returns the Playwright ws endpoint (wsEndpoint)",
        args: &[ID],
        flags: &[],
        examples: &["camoufox-manager profile start 3f9c2a7d1e8b4c5a6d7e8f90 --json", "# then: playwright.firefox.connect(wsEndpoint)"],
        needs_app: true,
    },
    Command {
        name: "profile stop",
        summary: "Close a profile's browser gracefully",
        args: &[ID],
        flags: &[],
        examples: &["camoufox-manager profile stop 3f9c2a7d1e8b4c5a6d7e8f90"],
        needs_app: true,
    },
    Command {
        name: "profile delete",
        summary: "Delete a profile and all its browser data (cannot be undone)",
        args: &[ID],
        flags: &[],
        examples: &["camoufox-manager profile delete 3f9c2a7d1e8b4c5a6d7e8f90"],
        needs_app: true,
    },
    Command {
        name: "proxy list",
        summary: "List the proxy library (passwords are never shown)",
        args: &[],
        flags: &[],
        examples: &["camoufox-manager proxy list --json"],
        needs_app: true,
    },
    Command {
        name: "proxy import",
        summary: "Import proxies from a file, stdin or inline text; the whole list is rejected if any line is invalid",
        args: &[],
        flags: &[
            flag("file", "<path>", "file with one proxy per line, `-` for stdin"),
            flag("text", "<lines>", "proxy lines given inline (newline or comma separated)"),
        ],
        examples: &[
            "camoufox-manager proxy import --file proxies.txt",
            "camoufox-manager proxy import --text \"socks5://user:pass@host:1080\"",
        ],
        needs_app: true,
    },
    Command {
        name: "proxy check",
        summary: "Check a proxy through the local relay: reachability, exit IP, country, timezone, latency",
        args: &[Arg { name: "id", summary: "proxy id (see `proxy list`)" }],
        flags: &[],
        examples: &["camoufox-manager proxy check 2 --json"],
        needs_app: true,
    },
    Command {
        name: "proxy assign",
        summary: "Assign one proxy to one or more (closed) profiles",
        args: &[Arg { name: "proxy-id", summary: "proxy id (see `proxy list`)" }],
        flags: &[Flag {
            name: "profile",
            value: Some("<id>"),
            summary: "profile id; repeat or comma-separate for several",
            required: true,
            repeatable: true,
        }],
        examples: &["camoufox-manager proxy assign 2 --profile 3f9c2a7d1e8b4c5a6d7e8f90,a1b2c3d4e5f60718293a4b5c"],
        needs_app: true,
    },
    Command {
        name: "doctor",
        summary: "Basic health check (placeholder: full diagnostics arrive in a later release)",
        args: &[],
        flags: &[],
        examples: &["camoufox-manager doctor --json"],
        needs_app: true,
    },
    Command {
        name: "skill",
        summary: "Print the agent skill document (SKILL.md) generated from these specs",
        args: &[],
        flags: &[],
        examples: &["camoufox-manager skill > ~/.claude/skills/camoufox-manager/SKILL.md"],
        needs_app: false,
    },
    Command {
        name: "docs",
        summary: "Print the markdown command reference generated from these specs",
        args: &[],
        flags: &[],
        examples: &["camoufox-manager docs > cli.md"],
        needs_app: false,
    },
];

pub fn find(words: &[&str]) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name.split(' ').count() <= words.len() && c.name.split(' ').zip(words).all(|(a, b)| a == *b))
}

fn flag_usage(f: &Flag) -> String {
    let body = match f.value {
        Some(v) => format!("--{} {v}", f.name),
        None => format!("--{}", f.name),
    };
    if f.required { body } else { format!("[{body}]") }
}

/// `camoufox-manager profile create --name <name> [--os <win|mac|lin>]`
pub fn usage(c: &Command) -> String {
    let mut s = format!("camoufox-manager {}", c.name);
    for a in c.args {
        s.push_str(&format!(" <{}>", a.name));
    }
    for f in c.flags {
        s.push(' ');
        s.push_str(&flag_usage(f));
    }
    s.push_str(" [--json]");
    s
}

fn flag_rows(flags: &[Flag]) -> String {
    let w = flags.iter().map(|f| flag_usage(f).len()).max().unwrap_or(0);
    flags
        .iter()
        .map(|f| format!("  {:<w$}  {}{}\n", flag_usage(f), f.summary, if f.repeatable { " (repeatable)" } else { "" }, w = w))
        .collect()
}

/// `camoufox-manager --help`
pub fn top_help() -> String {
    let mut s = String::from(
        "camoufox-manager - Camoufox profile manager\n\n\
         usage: camoufox-manager                 open the desktop window\n\
         \x20      camoufox-manager serve           run the server only (CM_PORT, default 7878)\n\
         \x20      camoufox-manager <command> ...   talk to the running app\n\n\
         commands:\n",
    );
    let w = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
    for c in COMMANDS {
        s.push_str(&format!("  {:<w$}  {}\n", c.name, c.summary, w = w));
    }
    s.push_str("\nglobal flags:\n");
    s.push_str(&flag_rows(GLOBAL_FLAGS));
    s.push_str(
        "\nenvironment: CM_PORT (app port), CM_API_KEY (API key)\n\
         errors: exit code 1, stderr `error: <code>: <message>`\n\
         `camoufox-manager <command> --help` for details; `camoufox-manager skill` for the agent skill.\n",
    );
    s
}

/// `camoufox-manager <command> --help`
pub fn command_help(c: &Command) -> String {
    let mut s = format!("{}\n\nusage: {}\n", c.summary, usage(c));
    if !c.args.is_empty() {
        s.push_str("\narguments:\n");
        for a in c.args {
            s.push_str(&format!("  <{}>  {}\n", a.name, a.summary));
        }
    }
    if !c.flags.is_empty() {
        s.push_str("\nflags:\n");
        s.push_str(&flag_rows(c.flags));
    }
    s.push_str("\nglobal flags: --json, --api-key <key>, --port <n>\n");
    if !c.examples.is_empty() {
        s.push_str("\nexamples:\n");
        for e in c.examples {
            s.push_str(&format!("  {e}\n"));
        }
    }
    s
}

fn command_markdown(c: &Command) -> String {
    let mut s = format!("### `{}`\n\n{}\n\n```\n{}\n```\n\n", c.name, c.summary, usage(c));
    if !c.args.is_empty() || !c.flags.is_empty() {
        for a in c.args {
            s.push_str(&format!("- `<{}>`: {}\n", a.name, a.summary));
        }
        for f in c.flags {
            let req = if f.required { " (required)" } else { "" };
            s.push_str(&format!(
                "- `{}`: {}{}{}\n",
                flag_usage(f).trim_matches(['[', ']']),
                f.summary,
                if f.repeatable { " (repeatable)" } else { "" },
                req
            ));
        }
        s.push('\n');
    }
    if !c.examples.is_empty() {
        s.push_str("```sh\n");
        for e in c.examples {
            s.push_str(e);
            s.push('\n');
        }
        s.push_str("```\n\n");
    }
    s
}

/// Markdown reference of every command.
pub fn markdown() -> String {
    let mut s =
        String::from("# camoufox-manager CLI\n\nGenerated from the command specs (`camoufox-manager docs`). Do not edit by hand.\n\n");
    s.push_str("## Global flags\n\n");
    for f in GLOBAL_FLAGS {
        s.push_str(&format!("- `{}`: {}\n", flag_usage(f).trim_matches(['[', ']']), f.summary));
    }
    s.push_str(
        "\nEnvironment: `CM_PORT` (port of the running app), `CM_API_KEY`.\n\
         Errors: exit code 1 and `error: <code>: <message>` on stderr (codes: `usage`, `unreachable`, `invalid_api_key`, \
         `api_key_required`, `forbidden_origin`, `not_found`, `invalid`, `conflict`, `not_implemented`, `error`).\n\n## Commands\n\n",
    );
    for c in COMMANDS {
        s.push_str(&command_markdown(c));
    }
    s
}

/// The agent skill file (SKILL.md): when to use the tool, the working conventions, the reference.
pub fn skill() -> String {
    let mut s = String::from(
        "---\n\
         name: camoufox-manager\n\
         description: Drive Camoufox Manager, a local anti-detect browser profile manager. Use when you need a persistent \
         browser profile with its own fingerprint and proxy: create or list profiles, assign proxies, start a profile and \
         get its Playwright ws endpoint, stop or delete it.\n\
         ---\n\n\
         # Camoufox Manager CLI\n\n\
         `camoufox-manager` controls the Camoufox Manager app running on this machine (127.0.0.1 only). \
         Each profile is a persistent Firefox (Camoufox) profile with a pinned hardware fingerprint, its own cookies and \
         optionally its own proxy. You drive the browser itself with Playwright over the ws endpoint the CLI returns.\n\n\
         ## Setup\n\n\
         1. The app must be running: the desktop window, or `CM_PORT=7878 camoufox-manager serve`. Use the same `CM_PORT` \
         for the CLI (default 7878).\n\
         2. If the owner enabled \"require API key\", pass a key: `export CM_API_KEY=cmk_...` (or `--api-key`). Keys are \
         created per agent/machine in Settings; the secret is shown once. Ask the owner for one if you get \
         `error: api_key_required` or `error: invalid_api_key`.\n\
         3. Camoufox itself is optional; `status` tells you whether the engine is ready. If not, ask the owner to install \
         it from Settings (do not try to install it yourself).\n\n\
         ## Conventions\n\n\
         - Always add `--json` and parse stdout; stderr carries only errors and progress.\n\
         - Errors exit non-zero with one line `error: <code>: <message>` on stderr. Codes: `usage`, `unreachable` (app not \
         running), `invalid_api_key`, `api_key_required`, `forbidden_origin`, `not_found`, `invalid`, `conflict` (e.g. profile \
         already running), `not_implemented`, `error`.\n\
         - Profile ids are 24 hex characters; proxy ids are integers.\n\n\
         ## Typical workflow\n\n\
         ```sh\n\
         camoufox-manager status --json\n\
         ID=$(camoufox-manager profile create --name \"task-1\" --os mac --json | jq -r .id)\n\
         camoufox-manager proxy list --json                       # pick a proxy id\n\
         camoufox-manager proxy assign 2 --profile $ID --json     # optional; the browser fails closed if the proxy is dead\n\
         WS=$(camoufox-manager profile start $ID --json | jq -r .wsEndpoint)\n\
         # in your code: browser = await playwright.firefox.connect(WS)\n\
         camoufox-manager profile stop $ID --json                 # when done\n\
         ```\n\n\
         Rules of thumb: one agent per profile at a time; stop the profile when you are done (cookies and logins are kept \
         in the profile); never delete a profile you did not create; proxy passwords are never exposed, do not try to read \
         them. `profile start` blocks until the browser is up (it first checks the proxy, which can take several seconds).\n\n\
         ## Command reference\n\n",
    );
    for c in COMMANDS.iter().filter(|c| c.needs_app) {
        s.push_str(&command_markdown(c));
    }
    s.push_str("Also: `camoufox-manager docs` (reference as markdown), `camoufox-manager skill` (this file), `--help` on any command.\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_matches_one_and_two_word_commands() {
        assert_eq!(find(&["status"]).unwrap().name, "status");
        assert_eq!(find(&["profile", "start", "x"]).unwrap().name, "profile start");
        assert!(find(&["profile"]).is_none());
        assert!(find(&["nope"]).is_none());
    }

    #[test]
    fn names_unique_and_documented() {
        for (i, c) in COMMANDS.iter().enumerate() {
            assert!(COMMANDS[i + 1..].iter().all(|d| d.name != c.name), "{} twice", c.name);
            assert!(!c.summary.is_empty() && !c.examples.is_empty(), "{}", c.name);
        }
    }

    #[test]
    fn generated_docs_cover_every_command() {
        let (md, skill, help) = (markdown(), skill(), top_help());
        for c in COMMANDS {
            assert!(md.contains(&format!("### `{}`", c.name)));
            assert!(help.contains(c.name));
            assert_eq!(skill.contains(&format!("### `{}`", c.name)), c.needs_app, "{}", c.name);
        }
        assert!(skill.starts_with("---\nname: camoufox-manager\n"));
        assert!(usage(find(&["profile", "create"]).unwrap()).contains("--name <name> [--os <win|mac|lin>] [--json]"));
    }
}
