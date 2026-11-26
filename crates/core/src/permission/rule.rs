//! Permission rule grammar: `tool-name` or `tool-name(pattern)`.

use std::fmt;
use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// A parsed permission rule.
///
/// The tool part and the optional argument pattern both use `*` (and `**`)
/// as "any run of characters, including `/`". `\*` matches a literal `*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    tool: String,
    pattern: Option<String>,
    text: String,
}

impl Rule {
    /// Build a rule from its parts (used for built-in rules).
    pub fn new(tool: &str, pattern: Option<&str>) -> Self {
        let text = match pattern {
            Some(p) => format!("{tool}({p})"),
            None => tool.to_string(),
        };
        Self {
            tool: tool.to_string(),
            pattern: pattern.map(str::to_string),
            text,
        }
    }

    /// The rule as written, e.g. `bash(git status*)`.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The tool part, for example `bash` for `bash(git status*)`.
    pub fn tool(&self) -> &str {
        &self.tool
    }

    /// The argument pattern, if any.
    pub fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    /// Whether the rule's tool part covers `tool_name`.
    pub fn matches_tool(&self, tool_name: &str) -> bool {
        wildcard_match(&self.tool, tool_name)
    }
}

impl FromStr for Rule {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            bail!("empty permission rule");
        }
        let Some(open) = s.find('(') else {
            return Ok(Self::new(s, None));
        };
        let Some(inner) = s.strip_suffix(')') else {
            bail!("permission rule '{s}' has an unclosed '('");
        };
        let tool = &inner[..open];
        let pattern = &inner[open + 1..];
        if tool.is_empty() {
            bail!("permission rule '{s}' has no tool name");
        }
        if tool.contains(|c: char| c.is_whitespace()) {
            bail!("permission rule '{s}' has whitespace in the tool name");
        }
        Ok(Self::new(tool, Some(pattern)))
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl Serialize for Rule {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.text)
    }
}

impl<'de> Deserialize<'de> for Rule {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// Anchored wildcard match. `*`/`**` match any run (including empty);
/// `\*` matches a literal `*`.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    #[derive(PartialEq)]
    enum Tok {
        Lit(char),
        Star,
    }
    let mut toks: Vec<Tok> = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'*') => {
                chars.next();
                toks.push(Tok::Lit('*'));
            }
            '*' => {
                if toks.last() != Some(&Tok::Star) {
                    toks.push(Tok::Star);
                }
            }
            c => toks.push(Tok::Lit(c)),
        }
    }

    let text: Vec<char> = text.chars().collect();
    let (mut t, mut p) = (0usize, 0usize);
    let (mut star_p, mut star_t): (Option<usize>, usize) = (None, 0);
    while t < text.len() {
        match toks.get(p) {
            Some(Tok::Lit(c)) if *c == text[t] => {
                p += 1;
                t += 1;
            }
            Some(Tok::Star) => {
                star_p = Some(p);
                star_t = t;
                p += 1;
            }
            _ => match star_p {
                Some(sp) => {
                    p = sp + 1;
                    star_t += 1;
                    t = star_t;
                }
                None => return false,
            },
        }
    }
    while matches!(toks.get(p), Some(Tok::Star)) {
        p += 1;
    }
    p == toks.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_tool() {
        let r: Rule = "bash".parse().unwrap();
        assert!(r.matches_tool("bash"));
        assert_eq!(r.pattern(), None);
    }

    #[test]
    fn tool_is_the_part_before_the_pattern() {
        let with_pattern: Rule = "bash(git status*)".parse().unwrap();
        let bare: Rule = "web-fetch".parse().unwrap();
        assert_eq!(with_pattern.tool(), "bash");
        assert_eq!(bare.tool(), "web-fetch");
    }

    #[test]
    fn parses_tool_with_pattern() {
        let r: Rule = "bash(git status*)".parse().unwrap();
        assert_eq!(r.pattern(), Some("git status*"));
        assert_eq!(r.as_str(), "bash(git status*)");
    }

    #[test]
    fn pattern_may_contain_parens() {
        let r: Rule = "bash(echo (a))".parse().unwrap();
        assert_eq!(r.pattern(), Some("echo (a)"));
    }

    #[test]
    fn rejects_malformed_rules() {
        for bad in ["", "  ", "bash(git", "(x)", "ba sh(x)"] {
            assert!(bad.parse::<Rule>().is_err(), "'{bad}' should be rejected");
        }
    }

    #[test]
    fn tool_part_supports_wildcards() {
        let r: Rule = "file-*".parse().unwrap();
        assert!(r.matches_tool("file-read"));
        assert!(!r.matches_tool("bash"));
    }

    #[test]
    fn wildcard_is_anchored_and_spans_slashes() {
        assert!(wildcard_match("git status*", "git status --short"));
        assert!(!wildcard_match("git status*", "echo git status"));
        assert!(wildcard_match("/w/**", "/w/a/b/c.txt"));
        assert!(!wildcard_match("/w/**", "/w"));
        assert!(wildcard_match("*://127.*", "http://127.0.0.1/"));
        assert!(wildcard_match("a*b*c", "aXXbYYc"));
        assert!(!wildcard_match("a*b*c", "aXXbYY"));
    }

    #[test]
    fn escaped_star_is_literal() {
        assert!(wildcard_match(r"rm -rf /\*", "rm -rf /*"));
        assert!(!wildcard_match(r"rm -rf /\*", "rm -rf /tmp"));
    }

    #[test]
    fn serde_round_trip_validates() {
        let r: Rule = serde_json::from_str("\"bash(ls*)\"").unwrap();
        assert_eq!(serde_json::to_string(&r).unwrap(), "\"bash(ls*)\"");
        assert!(serde_json::from_str::<Rule>("\"bash(ls\"").is_err());
    }
}
