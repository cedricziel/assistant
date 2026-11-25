//! Normalized view of the integrations.sh catalog.
//!
//! The upstream document is an unstable (v0.0.1) JSON feed of remote MCP
//! servers, OpenAPI specs, GraphQL endpoints and CLIs. [`normalize`] is the
//! single place that knows the upstream shape: it keeps only installable
//! remote MCP entries and maps them onto [`CatalogEntry`].

use serde::{Deserialize, Serialize};
use serde_json::Value;

// -- Types -------------------------------------------------------------------

/// One installable remote MCP server from the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    pub connect_url: String,
    pub auth: CatalogAuth,
}

/// How an entry authenticates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogAuth {
    pub kind: CatalogAuthKind,
    /// Header to send, for `header` and `mixed` entries whose template parsed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    /// Name of the `{placeholder}` the user fills in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Literal text before the secret, e.g. `"Bearer "`; empty when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_prefix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CatalogAuthKind {
    None,
    Header,
    Oauth,
    Mixed,
    Unknown,
}

// -- Normalization -----------------------------------------------------------

/// Map one raw upstream entry onto [`CatalogEntry`].
///
/// Returns `None` for entries that are not installable remote MCP servers:
/// another `kind`, a missing `id`, `name` or `connectUrl`, or a connect URL
/// that is not `https`. Unknown fields are ignored; optional fields of the
/// wrong type fall back to empty.
pub fn normalize(raw: &Value) -> Option<CatalogEntry> {
    if str_field(raw, "kind")? != "mcp" {
        return None;
    }
    let connect_url = str_field(raw, "connectUrl")?;
    if !is_https_url(connect_url) {
        return None;
    }
    Some(CatalogEntry {
        id: str_field(raw, "id")?.to_owned(),
        name: str_field(raw, "name")?.to_owned(),
        description: str_field(raw, "description").unwrap_or_default().to_owned(),
        domain: str_field(raw, "domain").unwrap_or_default().to_owned(),
        icon_url: str_field(raw, "icon").map(str::to_owned),
        categories: raw
            .get("categories")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|c| c.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        connect_url: connect_url.to_owned(),
        auth: normalize_auth(raw.get("auth")),
    })
}

/// Non-empty string value of `key`, if `raw` is an object that has one.
fn str_field<'a>(raw: &'a Value, key: &str) -> Option<&'a str> {
    raw.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn is_https_url(url: &str) -> bool {
    let has_https_scheme = url
        .get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
    has_https_scheme && url.len() > 8 && !url.contains(char::is_whitespace)
}

fn normalize_auth(raw: Option<&Value>) -> CatalogAuth {
    let kind = raw.and_then(|a| str_field(a, "kind"));
    let kind = match kind {
        Some("none") => CatalogAuthKind::None,
        Some("oauth") => CatalogAuthKind::Oauth,
        Some("api_key" | "token") => CatalogAuthKind::Header,
        Some("mixed") => CatalogAuthKind::Mixed,
        _ => CatalogAuthKind::Unknown,
    };
    let template = match kind {
        CatalogAuthKind::Header | CatalogAuthKind::Mixed => raw
            .and_then(|a| str_field(a, "header"))
            .and_then(parse_header_template),
        _ => None,
    };
    match template {
        Some(t) => CatalogAuth {
            kind,
            header_name: Some(t.header_name),
            placeholder: Some(t.placeholder),
            value_prefix: Some(t.value_prefix),
        },
        None => CatalogAuth {
            kind,
            header_name: None,
            placeholder: None,
            value_prefix: None,
        },
    }
}

struct HeaderTemplate {
    header_name: String,
    placeholder: String,
    value_prefix: String,
}

/// Parse `Name: [prefix ]{placeholder}`, e.g. `Authorization: Bearer {pat}`.
fn parse_header_template(template: &str) -> Option<HeaderTemplate> {
    let (name, value) = template.split_once(':')?;
    let header_name = name.trim();
    if header_name.is_empty() || header_name.contains(char::is_whitespace) {
        return None;
    }
    let value = value.trim_start();
    let open = value.find('{')?;
    let close = open + value[open..].find('}')?;
    let placeholder = &value[open + 1..close];
    if placeholder.is_empty() || !value[close + 1..].trim().is_empty() {
        return None;
    }
    Some(HeaderTemplate {
        header_name: header_name.to_owned(),
        placeholder: placeholder.to_owned(),
        value_prefix: value[..open].to_owned(),
    })
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FIXTURE: &str = include_str!("../../tests/fixtures/integrations-catalog.json");

    fn fixture_entries() -> Vec<Value> {
        let doc: Value = serde_json::from_str(FIXTURE).unwrap();
        doc["data"].as_array().unwrap().clone()
    }

    fn normalized(id: &str) -> Option<CatalogEntry> {
        let entries = fixture_entries();
        let raw = entries
            .iter()
            .find(|e| e["id"] == id)
            .unwrap_or_else(|| panic!("fixture has no entry {id}"));
        normalize(raw)
    }

    fn header_auth(
        name: &str,
        placeholder: &str,
        prefix: &str,
        kind: CatalogAuthKind,
    ) -> CatalogAuth {
        CatalogAuth {
            kind,
            header_name: Some(name.into()),
            placeholder: Some(placeholder.into()),
            value_prefix: Some(prefix.into()),
        }
    }

    fn mcp_with_auth(auth: Value) -> Value {
        json!({
            "id": "x/y", "kind": "mcp", "name": "Y",
            "connectUrl": "https://y.example/mcp", "auth": auth
        })
    }

    #[test]
    fn fixture_keeps_only_https_mcp_entries() {
        let mut kept: Vec<String> = fixture_entries()
            .iter()
            .filter_map(normalize)
            .map(|e| e.id)
            .collect();
        kept.sort();

        let mut expected: Vec<String> = [
            "curated/axiom-co-mcp",
            "curated/deepwiki-com-mcp",
            "curated/exa-ai-mcp",
            "curated/linear-app-mcp",
            "curated/notion-com-mcp",
            "curated/sentry-io-mcp",
            "mcp/atlassian",
            "mcp/cloudflare",
            "mcp/miro",
            "curated/figma-com-mcp",
            "curated/posthog-com-mcp",
            "curated/github-com-mcp",
            "curated/stripe-com-mcp",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        expected.sort();

        assert_eq!(kept, expected);
    }

    #[test]
    fn non_mcp_kinds_are_skipped() {
        for id in [
            "curated/exa",
            "curated/anilist-co-graphql",
            "curated/github-com-cli",
        ] {
            assert_eq!(normalized(id), None, "{id} must be skipped");
        }
    }

    #[test]
    fn mcp_entry_without_connect_url_is_skipped() {
        for id in [
            "curated/resend-com-mcp",
            "discovered/agent-guild-5d5r-onrender-com-mcp",
        ] {
            assert_eq!(normalized(id), None, "{id} must be skipped");
        }
    }

    #[test]
    fn non_https_connect_urls_are_skipped() {
        for id in [
            "discovered/kakao-golf-mcp",
            "discovered/aside-com-mcp",
            "discovered/hookdeck-com-mcp",
        ] {
            assert_eq!(normalized(id), None, "{id} must be skipped");
        }
    }

    #[test]
    fn entry_missing_a_required_field_is_skipped() {
        assert_eq!(normalized("crafted/missing-name-mcp"), None);
    }

    #[test]
    fn plain_fields_are_mapped() {
        let linear = normalized("curated/linear-app-mcp").unwrap();

        assert_eq!(linear.id, "curated/linear-app-mcp");
        assert_eq!(linear.name, "Linear");
        assert_eq!(linear.domain, "linear.app");
        assert_eq!(linear.connect_url, "https://mcp.linear.app/mcp");
        assert_eq!(
            linear.icon_url.as_deref(),
            Some("https://www.google.com/s2/favicons?domain=linear.app&sz=64")
        );
        assert_eq!(linear.categories, ["productivity", "project-management"]);
        assert!(!linear.description.is_empty());
    }

    #[test]
    fn oauth_and_none_kinds_carry_no_header() {
        let oauth = normalized("curated/linear-app-mcp").unwrap().auth;
        assert_eq!(oauth.kind, CatalogAuthKind::Oauth);
        assert_eq!(oauth.header_name, None);

        let none = normalized("curated/deepwiki-com-mcp").unwrap().auth;
        assert_eq!(none.kind, CatalogAuthKind::None);
        assert_eq!(none.header_name, None);
    }

    #[test]
    fn api_key_becomes_header_with_bearer_prefix() {
        let auth = normalized("curated/posthog-com-mcp").unwrap().auth;

        assert_eq!(
            auth,
            header_auth(
                "Authorization",
                "phx_key",
                "Bearer ",
                CatalogAuthKind::Header
            )
        );
    }

    #[test]
    fn mixed_keeps_the_parsed_header() {
        let auth = normalized("curated/github-com-mcp").unwrap().auth;

        assert_eq!(
            auth,
            header_auth("Authorization", "pat", "Bearer ", CatalogAuthKind::Mixed)
        );
    }

    #[test]
    fn missing_or_null_auth_is_unknown() {
        // cloudflare has `"auth": null`, miro has no `auth` key at all.
        for id in ["mcp/cloudflare", "mcp/miro"] {
            let auth = normalized(id).unwrap().auth;
            assert_eq!(auth.kind, CatalogAuthKind::Unknown, "{id}");
            assert_eq!(auth.header_name, None, "{id}");
        }
    }

    #[test]
    fn x_api_key_template_is_parsed_without_prefix() {
        let raw = mcp_with_auth(json!({"kind": "api_key", "header": "x-api-key: {api_key}"}));

        assert_eq!(
            normalize(&raw).unwrap().auth,
            header_auth("x-api-key", "api_key", "", CatalogAuthKind::Header)
        );
    }

    #[test]
    fn token_kind_maps_to_header() {
        let raw = mcp_with_auth(json!({"kind": "token", "header": "Authorization: Bearer {pat}"}));

        assert_eq!(
            normalize(&raw).unwrap().auth,
            header_auth("Authorization", "pat", "Bearer ", CatalogAuthKind::Header)
        );
    }

    #[test]
    fn header_kind_without_a_template_has_no_header_name() {
        let raw = mcp_with_auth(json!({"kind": "api_key"}));

        let auth = normalize(&raw).unwrap().auth;

        assert_eq!(auth.kind, CatalogAuthKind::Header);
        assert_eq!(auth.header_name, None);
        assert_eq!(auth.placeholder, None);
    }

    #[test]
    fn malformed_header_templates_yield_no_header_name() {
        for template in [
            "",
            "no-colon {x}",
            "X-Key: no placeholder",
            "X-Key: {a} trailing",
            ": {a}",
            "Bad Name: {a}",
        ] {
            let raw = mcp_with_auth(json!({"kind": "api_key", "header": template}));

            let auth = normalize(&raw).unwrap().auth;

            assert_eq!(auth.kind, CatalogAuthKind::Header, "template {template:?}");
            assert_eq!(auth.header_name, None, "template {template:?}");
        }
    }

    #[test]
    fn unrecognized_auth_kind_is_unknown() {
        let raw = mcp_with_auth(json!({"kind": "kerberos"}));

        assert_eq!(normalize(&raw).unwrap().auth.kind, CatalogAuthKind::Unknown);
    }

    #[test]
    fn unknown_fields_and_wrong_typed_optionals_are_tolerated() {
        let raw = json!({
            "id": "x/y", "kind": "mcp", "name": "Y",
            "connectUrl": "https://y.example/mcp",
            "futureField": {"a": 1}, "categories": "not-an-array", "icon": 7,
            "description": null
        });

        let entry = normalize(&raw).unwrap();

        assert_eq!(entry.categories, Vec::<String>::new());
        assert_eq!(entry.icon_url, None);
        assert_eq!(entry.description, "");
    }

    #[test]
    fn non_object_input_is_skipped() {
        assert_eq!(normalize(&json!("mcp")), None);
        assert_eq!(normalize(&Value::Null), None);
    }

    #[test]
    fn header_auth_serializes_to_the_spec_shape() {
        let raw = mcp_with_auth(json!({"kind": "api_key", "header": "x-api-key: {api_key}"}));

        let value = serde_json::to_value(normalize(&raw).unwrap().auth).unwrap();

        assert_eq!(
            value,
            json!({
                "kind": "header",
                "header_name": "x-api-key",
                "placeholder": "api_key",
                "value_prefix": ""
            })
        );
    }
}
