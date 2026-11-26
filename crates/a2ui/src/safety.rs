//! Pure safety checks: regex backtracking safety and URL policy.

use regex_syntax::ast::{Ast, parse::Parser};
use url::Url;
use uuid::Uuid;

use crate::validate::UrlPolicy;

pub(crate) const ATTACHMENT_PREFIX: &str = "/api/attachments/";
const MAX_PATTERN_CHARS: usize = 200;

// -- Regex --

/// Why a `regex` pattern is rejected, or `None` when it is safe for a
/// backtracking client engine.
///
/// The Rust engine is linear-time, so compiling proves nothing about the
/// client's backtracking `RegExp`. The check walks the parsed syntax tree:
/// no quantified group may contain another quantifier or an alternation.
pub(crate) fn regex_problem(pattern: &str) -> Option<String> {
    if pattern.chars().count() > MAX_PATTERN_CHARS {
        return Some(format!(
            "pattern is longer than {MAX_PATTERN_CHARS} characters"
        ));
    }
    if let Err(e) = regex::Regex::new(pattern) {
        return Some(format!("pattern does not compile: {e}"));
    }
    let ast = match Parser::new().parse(pattern) {
        Ok(ast) => ast,
        Err(e) => return Some(format!("pattern does not parse: {e}")),
    };
    backtracking_risk(&ast).map(str::to_owned)
}

fn backtracking_risk(ast: &Ast) -> Option<&'static str> {
    match ast {
        Ast::Repetition(r) if contains(&r.ast, &|a| matches!(a, Ast::Repetition(_))) => {
            Some("a quantified group contains another quantifier")
        }
        Ast::Repetition(r) if contains(&r.ast, &|a| matches!(a, Ast::Alternation(_))) => {
            Some("a quantified group contains an alternation")
        }
        Ast::Repetition(r) => backtracking_risk(&r.ast),
        Ast::Group(g) => backtracking_risk(&g.ast),
        Ast::Alternation(a) => a.asts.iter().find_map(backtracking_risk),
        Ast::Concat(c) => c.asts.iter().find_map(backtracking_risk),
        _ => None,
    }
}

/// Whether `ast` or any node below it satisfies `pred`.
fn contains(ast: &Ast, pred: &dyn Fn(&Ast) -> bool) -> bool {
    pred(ast)
        || match ast {
            Ast::Repetition(r) => contains(&r.ast, pred),
            Ast::Group(g) => contains(&g.ast, pred),
            Ast::Alternation(a) => a.asts.iter().any(|x| contains(x, pred)),
            Ast::Concat(c) => c.asts.iter().any(|x| contains(x, pred)),
            _ => false,
        }
}

// -- URLs --

/// The attachment id in a same-origin `/api/attachments/{id}` path, if the
/// string has that shape.
pub(crate) fn attachment_id(url: &str) -> Option<Uuid> {
    Uuid::parse_str(url.strip_prefix(ATTACHMENT_PREFIX)?).ok()
}

/// An `Image.url` target: `https:` (optionally host-restricted) or an owned
/// same-origin attachment path.
pub(crate) fn check_image_url(url: &str, policy: &UrlPolicy) -> Result<(), String> {
    if url.starts_with(ATTACHMENT_PREFIX) {
        let id = attachment_id(url).ok_or("malformed attachment path")?;
        return if policy.allowed_attachment_ids.contains(&id) {
            Ok(())
        } else {
            Err(format!(
                "attachment {id} does not belong to this conversation"
            ))
        };
    }
    let parsed = https_url(url)?;
    let host = parsed.host_str().ok_or("URL has no host")?;
    match policy.image_hosts.as_deref() {
        Some(hosts) if !hosts.is_empty() => {
            if hosts.iter().any(|h| h.eq_ignore_ascii_case(host)) {
                Ok(())
            } else {
                Err(format!(
                    "image host {host} is not in the allowed image hosts"
                ))
            }
        }
        _ => Ok(()),
    }
}

/// An `openUrl` target: `https:` or `mailto:`.
pub(crate) fn check_link_url(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| format!("not an absolute URL: {e}"))?;
    match parsed.scheme() {
        "https" | "mailto" => Ok(()),
        other => Err(format!(
            "scheme {other}: is not allowed, use https: or mailto:"
        )),
    }
}

fn https_url(url: &str) -> Result<Url, String> {
    let parsed = Url::parse(url).map_err(|e| format!("not an absolute URL: {e}"))?;
    if parsed.scheme() == "https" {
        Ok(parsed)
    } else {
        Err(format!(
            "scheme {}: is not allowed, use https:",
            parsed.scheme()
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use uuid::Uuid;

    use super::*;

    fn policy(hosts: Option<&[&str]>, owned: &[Uuid]) -> UrlPolicy {
        UrlPolicy {
            allowed_attachment_ids: owned.iter().copied().collect::<HashSet<_>>(),
            image_hosts: hosts.map(|h| h.iter().map(|s| (*s).to_owned()).collect()),
        }
    }

    #[test]
    fn regex_accepts_simple_patterns() {
        for p in [
            "^[0-9]{5}$",
            r"^\d+$",
            "^(ab)+$",
            "^[a-z]+@[a-z]+$",
            "(?:ab){2}",
        ] {
            assert_eq!(regex_problem(p), None, "{p} should be accepted");
        }
    }

    #[test]
    fn regex_rejects_nested_quantifiers_and_quantified_alternation() {
        for p in [
            "(a+)+$",
            "(a*)*",
            r"(\w+\s?)*",
            "(a|aa)+",
            "(?:a|b){2,}",
            "((a+)b)+",
        ] {
            assert!(regex_problem(p).is_some(), "{p} should be rejected");
        }
    }

    #[test]
    fn regex_rejects_long_and_invalid_patterns() {
        assert!(regex_problem(&"a".repeat(201)).is_some());
        assert_eq!(regex_problem(&"a".repeat(200)), None);
        assert!(regex_problem("(unclosed").is_some());
        assert!(regex_problem(r"(a)\1").is_some(), "backreference");
    }

    #[test]
    fn image_urls_follow_the_scheme_host_and_attachment_rules() {
        let id = Uuid::new_v4();
        let open = policy(None, &[id]);
        assert!(check_image_url("https://any.example/p.png", &open).is_ok());
        assert!(check_image_url(&format!("/api/attachments/{id}"), &open).is_ok());
        for bad in [
            "javascript:alert(1)",
            "data:image/png;base64,AAAA",
            "http://a.example/p.png",
            "file:///etc/passwd",
            "//a.example/p.png",
            "not a url",
        ] {
            assert!(check_image_url(bad, &open).is_err(), "{bad}");
        }
        let other = Uuid::new_v4();
        assert!(check_image_url(&format!("/api/attachments/{other}"), &open).is_err());
        assert!(check_image_url("/api/attachments/not-a-uuid", &open).is_err());

        let listed = policy(Some(&["Images.Example.com"]), &[]);
        assert!(check_image_url("https://images.example.com/a.png", &listed).is_ok());
        let err = check_image_url("https://tracker.example.net/p.png", &listed)
            .expect_err("host outside the list");
        assert!(err.contains("tracker.example.net"), "{err}");

        let empty = policy(Some(&[]), &[]);
        assert!(check_image_url("https://any.example/p.png", &empty).is_ok());
    }

    #[test]
    fn link_urls_allow_https_and_mailto_only() {
        assert!(check_link_url("https://example.com/x").is_ok());
        assert!(check_link_url("mailto:a@example.com").is_ok());
        for bad in [
            "javascript:alert(1)",
            "data:text/html,x",
            "http://a.example",
            "file:///x",
            "x",
        ] {
            assert!(check_link_url(bad).is_err(), "{bad}");
        }
    }
}
