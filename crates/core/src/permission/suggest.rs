//! Rule suggestions for approval prompts.
//!
//! When a call needs a human, the prompt may offer "always allow" and "always
//! reject". [`suggest_rule`] derives the rule such a choice would save. It is
//! pure and errs narrow: a rule that is too broad would permit arbitrary code
//! or data loss in every later turn, including unattended ones. When no safe
//! rule exists it returns `None`, and the caller offers allow-once and
//! reject-once only.

use serde_json::Value;
use url::Url;

use super::rule::Rule;
use super::static_rules::SHELL_META;

/// Shells, interpreters and wrappers that run other commands. A saved rule for
/// one of these would allow anything. `python*` is matched as a prefix.
const COMMAND_RUNNERS: [&str; 20] = [
    "sh", "bash", "zsh", "fish", "dash", "python", "node", "deno", "bun", "ruby", "perl", "php",
    "lua", "env", "xargs", "eval", "exec", "nohup", "time", "watch",
];

/// Commands whose misuse destroys data or changes permissions.
const DESTRUCTIVE: [&str; 7] = ["rm", "mv", "chmod", "chown", "dd", "truncate", "shred"];

/// The rule an "always" choice would save for this call, or `None` when
/// neither "always" option may be offered.
pub fn suggest_rule(tool: &str, params: &Value) -> Option<Rule> {
    match tool {
        "bash" => suggest_bash(params.get("command")?.as_str()?),
        "web-fetch" => suggest_web_fetch(params.get("url")?.as_str()?),
        _ => suggest_tool(tool),
    }
}

/// `bash(<first token>[ <leading flags>][ <first non-flag token>]*)`.
fn suggest_bash(command: &str) -> Option<Rule> {
    let command = command.trim();
    if command.contains(SHELL_META) {
        return None;
    }
    let mut tokens = command.split_whitespace();
    let program = tokens.next()?;
    if is_command_runner(program) || is_destructive(program) || program.contains('=') {
        return None;
    }

    let mut subject = program.to_string();
    for token in tokens {
        subject.push(' ');
        subject.push_str(token);
        if !token.starts_with('-') {
            break;
        }
    }
    // `\` before the appended `*` would turn the wildcard into a literal star.
    if subject.ends_with('\\') {
        return None;
    }
    Some(Rule::new(
        "bash",
        Some(&format!("{}*", escape_stars(&subject))),
    ))
}

/// `web-fetch(<scheme>://<host>[:<port>]/*)` for http and https URLs.
fn suggest_web_fetch(url: &str) -> Option<Rule> {
    let url = Url::parse(url).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?;
    let authority = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    let pattern = format!("{}://{}/*", url.scheme(), escape_stars(&authority));
    Some(Rule::new("web-fetch", Some(&pattern)))
}

/// The bare tool name, for every tool without a finer-grained derivation.
fn suggest_tool(tool: &str) -> Option<Rule> {
    if tool.is_empty() || tool.contains(|c: char| c.is_whitespace() || c == '(' || c == ')') {
        return None;
    }
    Some(Rule::new(&escape_stars(tool), None))
}

fn is_command_runner(program: &str) -> bool {
    let name = basename(program);
    COMMAND_RUNNERS.contains(&name) || name.starts_with("python")
}

fn is_destructive(program: &str) -> bool {
    DESTRUCTIVE.contains(&basename(program))
}

fn basename(program: &str) -> &str {
    program.rsplit('/').next().unwrap_or(program)
}

/// `*` is a wildcard in rule patterns; `\*` matches a literal star.
fn escape_stars(text: &str) -> String {
    text.replace('*', "\\*")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::permission::wildcard_match;

    fn bash(command: &str) -> Option<String> {
        suggest_rule("bash", &json!({ "command": command })).map(|r| r.to_string())
    }

    #[test]
    fn bash_subcommand_keeps_the_first_non_flag_token() {
        assert_eq!(
            bash("git status --short").as_deref(),
            Some("bash(git status*)")
        );
    }

    #[test]
    fn bash_keeps_every_leading_flag() {
        assert_eq!(
            bash("npm -s run build").as_deref(),
            Some("bash(npm -s run*)")
        );
        assert_eq!(
            bash("cargo --locked -q test -p x").as_deref(),
            Some("bash(cargo --locked -q test*)")
        );
    }

    #[test]
    fn bash_without_a_non_flag_token_keeps_the_flags() {
        assert_eq!(bash("ls -la").as_deref(), Some("bash(ls -la*)"));
        assert_eq!(bash("  pwd ").as_deref(), Some("bash(pwd*)"));
    }

    #[test]
    fn bash_suggestion_matches_the_command_it_came_from() {
        for command in [
            "git status --short",
            "npm -s run build",
            "ls -la",
            "echo a*b",
        ] {
            let rule = suggest_rule("bash", &json!({ "command": command })).unwrap();
            assert!(
                wildcard_match(rule.pattern().unwrap(), command),
                "{rule} must match {command}"
            );
        }
    }

    #[test]
    fn literal_stars_are_escaped() {
        assert_eq!(
            bash("echo a*b").as_deref(),
            Some(r"bash(echo a\*b*)"),
            "a star in the command must not become a wildcard"
        );
    }

    #[test]
    fn a_trailing_backslash_cannot_swallow_the_wildcard() {
        assert_eq!(
            bash(r"echo foo\"),
            None,
            r"`foo\*` would match a literal star"
        );
    }

    #[test]
    fn bash_with_shell_metacharacters_has_no_suggestion() {
        for command in [
            "cat a | sh",
            "git status; rm x",
            "make && make test",
            "echo $HOME",
            "echo `id`",
            "echo $(id)",
            "cat < in",
            "echo hi > out",
            "ls\nrm x",
            "sleep 1 &",
        ] {
            assert_eq!(bash(command), None, "{command}");
        }
    }

    #[test]
    fn every_shell_interpreter_and_wrapper_has_no_suggestion() {
        for name in COMMAND_RUNNERS {
            assert_eq!(bash(&format!("{name} x")), None, "{name}");
            assert_eq!(bash(&format!("{name} -c x")), None, "{name} -c");
            assert_eq!(bash(name), None, "bare {name}");
            assert_eq!(bash(&format!("/usr/bin/{name} x")), None, "path to {name}");
        }
        for name in ["python3", "python3.12", "python2"] {
            assert_eq!(bash(&format!("{name} -c 'print(1)'")), None, "{name}");
        }
    }

    #[test]
    fn spec_examples_for_interpreters_and_wrappers() {
        for command in [
            r#"python3 -c "print(1)""#,
            r#"node -e "1""#,
            "sh -c ls",
            r#"bash -lc "make""#,
            "env FOO=1 make build",
            "xargs rm",
        ] {
            assert_eq!(bash(command), None, "{command}");
        }
    }

    #[test]
    fn every_destructive_command_has_no_suggestion() {
        for name in DESTRUCTIVE {
            assert_eq!(bash(&format!("{name} -rf build")), None, "{name}");
            assert_eq!(bash(name), None, "bare {name}");
            assert_eq!(bash(&format!("/bin/{name} x")), None, "path to {name}");
        }
        assert_eq!(bash("rm -rf build"), None);
    }

    #[test]
    fn a_leading_variable_assignment_is_a_wrapper() {
        assert_eq!(bash("FOO=1 make build"), None);
    }

    #[test]
    fn empty_or_missing_commands_have_no_suggestion() {
        assert_eq!(bash(""), None);
        assert_eq!(bash("   "), None);
        assert_eq!(suggest_rule("bash", &json!({})), None);
        assert_eq!(suggest_rule("bash", &json!({ "command": 7 })), None);
    }

    #[test]
    fn web_fetch_suggests_the_host() {
        let rule = suggest_rule(
            "web-fetch",
            &json!({ "url": "https://docs.rs/tokio/latest/tokio/" }),
        );
        assert_eq!(
            rule.map(|r| r.to_string()).as_deref(),
            Some("web-fetch(https://docs.rs/*)")
        );
    }

    #[test]
    fn web_fetch_keeps_a_non_default_port() {
        let rule = suggest_rule("web-fetch", &json!({ "url": "http://localhost:8080/x" }));
        assert_eq!(
            rule.map(|r| r.to_string()).as_deref(),
            Some("web-fetch(http://localhost:8080/*)")
        );
    }

    #[test]
    fn web_fetch_without_a_usable_url_has_no_suggestion() {
        for params in [
            json!({}),
            json!({ "url": "not a url" }),
            json!({ "url": "file:///etc/passwd" }),
            json!({ "url": "ftp://example.com/x" }),
        ] {
            assert_eq!(suggest_rule("web-fetch", &params), None, "{params}");
        }
    }

    #[test]
    fn other_tools_fall_back_to_the_bare_tool_name() {
        for tool in ["file-write", "web-search", "mcp__github__create_issue"] {
            let rule = suggest_rule(tool, &json!({ "path": "/x" }));
            assert_eq!(rule.map(|r| r.to_string()).as_deref(), Some(tool));
        }
    }

    #[test]
    fn a_tool_name_that_is_not_a_plain_name_has_no_suggestion() {
        for tool in ["", "has space", "a(b)"] {
            assert_eq!(suggest_rule(tool, &json!({})), None, "{tool:?}");
        }
    }

    #[test]
    fn suggestions_parse_back_to_the_same_rule() {
        for command in ["git status --short", "echo a*b", "ls -la"] {
            let rule = suggest_rule("bash", &json!({ "command": command })).unwrap();
            let reparsed: Rule = rule.as_str().parse().unwrap();
            assert_eq!(reparsed, rule);
        }
    }
}
