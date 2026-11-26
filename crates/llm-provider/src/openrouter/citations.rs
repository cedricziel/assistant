//! Turns `url_citation` annotations into a Sources list under the answer.

use serde_json::Value;

/// The Sources block for `answer`: each URL cited in `annotations`, once,
/// in order of first citation, linked under its title (or the URL). URLs
/// the answer already links inline are left out. `None` when nothing is
/// left to list.
pub fn sources_block(answer: &str, annotations: &[Value]) -> Option<String> {
    let mut seen: Vec<&str> = Vec::new();
    let mut lines = Vec::new();
    for citation in annotations
        .iter()
        .filter(|a| a["type"] == "url_citation")
        .map(|a| &a["url_citation"])
    {
        let Some(url) = citation["url"].as_str().filter(|u| !u.is_empty()) else {
            continue;
        };
        if seen.contains(&url) || answer.contains(&format!("]({url})")) {
            continue;
        }
        seen.push(url);
        let title = citation["title"]
            .as_str()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or(url);
        lines.push(format!("- [{}]({url})", escape_label(title)));
    }
    (!lines.is_empty()).then(|| format!("\n\nSources:\n{}", lines.join("\n")))
}

/// Escapes the brackets that would end a markdown link label.
fn escape_label(title: &str) -> String {
    title.replace('[', "\\[").replace(']', "\\]")
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::sources_block;

    fn cite(url: &str, title: Option<&str>) -> Value {
        let mut citation = json!({"url": url});
        if let Some(title) = title {
            citation["title"] = json!(title);
        }
        json!({"type": "url_citation", "url_citation": citation})
    }

    #[test]
    fn distinct_urls_in_order_of_first_citation() {
        let annotations = [
            cite("https://a.example", Some("A page")),
            cite("https://b.example", Some("B page")),
            cite("https://a.example", Some("A page")),
        ];
        assert_eq!(
            sources_block("Answer.", &annotations).unwrap(),
            "\n\nSources:\n- [A page](https://a.example)\n- [B page](https://b.example)"
        );
    }

    #[test]
    fn missing_title_uses_the_url() {
        let block = sources_block("Answer.", &[cite("https://a.example/x", None)]).unwrap();
        assert!(
            block.ends_with("- [https://a.example/x](https://a.example/x)"),
            "{block}"
        );
    }

    #[test]
    fn urls_linked_inline_are_not_listed_again() {
        let answer = "See [the docs](https://a.example) for more.";
        let annotations = [
            cite("https://a.example", Some("A")),
            cite("https://b.example", Some("B")),
        ];
        assert_eq!(
            sources_block(answer, &annotations).unwrap(),
            "\n\nSources:\n- [B](https://b.example)"
        );
        assert_eq!(
            sources_block(answer, &annotations[..1]),
            None,
            "nothing new to list"
        );
    }

    #[test]
    fn other_annotations_and_brackets_in_titles() {
        let annotations = [
            json!({"type": "file", "file": {"name": "x"}}),
            cite("https://a.example", Some("Release [beta] notes")),
        ];
        assert_eq!(
            sources_block("Answer.", &annotations).unwrap(),
            "\n\nSources:\n- [Release \\[beta\\] notes](https://a.example)"
        );
        assert_eq!(sources_block("Answer.", &[]), None);
    }
}
