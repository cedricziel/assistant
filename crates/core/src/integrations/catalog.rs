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
    !url.contains(char::is_whitespace)
        && url::Url::parse(url)
            .is_ok_and(|parsed| parsed.scheme() == "https" && is_plain_url(&parsed))
}

/// Whether `url` names a host and carries no userinfo or fragment. Userinfo
/// would put a secret where everyone who can see the integration reads it, and
/// a fragment never reaches the server.
pub fn is_plain_url(url: &url::Url) -> bool {
    url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
}

/// Lowercase `text` one character at a time.
///
/// `str::to_lowercase` picks a final sigma by context, so `ΟΣ` and `ΟΣΑ`
/// would fold `Σ` differently and a prefix query would miss.
fn fold(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

/// RFC 9110 `token`: the characters allowed in an HTTP field name.
fn is_http_token(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c))
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
    CatalogAuth {
        kind,
        header_name: template.as_ref().map(|t| t.header_name.clone()),
        placeholder: template.as_ref().map(|t| t.placeholder.clone()),
        value_prefix: template.map(|t| t.value_prefix),
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
    if !is_http_token(header_name) {
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

// -- Index -------------------------------------------------------------------

/// Page size when a query names none.
pub const DEFAULT_PAGE_SIZE: usize = 50;
/// Largest page a query may ask for.
pub const MAX_PAGE_SIZE: usize = 200;

/// Search parameters for [`CatalogIndex::search`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CatalogQuery<'a> {
    /// Case-insensitive text matched against name, domain and description.
    pub q: Option<&'a str>,
    /// Case-insensitive exact category.
    pub category: Option<&'a str>,
    /// Page size; defaults to [`DEFAULT_PAGE_SIZE`], capped at [`MAX_PAGE_SIZE`].
    pub limit: Option<usize>,
    /// `next_cursor` of the previous page.
    pub cursor: Option<&'a str>,
}

/// One page of search results.
#[derive(Debug)]
pub struct CatalogPage<'a> {
    pub items: Vec<&'a CatalogEntry>,
    /// Pass as `cursor` to get the next page; `None` on the last page.
    pub next_cursor: Option<String>,
}

/// The cursor is not one this index handed out.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidCursor;

impl std::fmt::Display for InvalidCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid cursor")
    }
}

impl std::error::Error for InvalidCursor {}

/// Normalized catalog entries with a precomputed search string each.
///
/// The catalog holds about 1,350 entries, so search is a linear scan.
#[derive(Debug, Default)]
pub struct CatalogIndex {
    entries: Vec<IndexedEntry>,
}

#[derive(Debug)]
struct IndexedEntry {
    entry: CatalogEntry,
    /// Lowercased `name`, `domain` and `description`, newline-separated.
    haystack: String,
}

impl IndexedEntry {
    fn new(entry: CatalogEntry) -> Self {
        let haystack = fold(&format!(
            "{}\n{}\n{}",
            entry.name, entry.domain, entry.description
        ));
        Self { entry, haystack }
    }

    /// `q` and `category` must already be [`fold`]ed.
    fn matches(&self, q: &str, category: Option<&str>) -> bool {
        self.haystack.contains(q)
            && category.is_none_or(|wanted| self.entry.categories.iter().any(|c| fold(c) == wanted))
    }
}

impl CatalogIndex {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build an index from an upstream document `{version, generatedAt, data[]}`.
    ///
    /// Returns `None` when `raw` is not a catalog (no `data` array). Entries
    /// that do not normalize are skipped.
    pub fn from_document(raw: &Value) -> Option<Self> {
        let data = raw.get("data")?.as_array()?;
        let entries = data
            .iter()
            .filter_map(normalize)
            .map(IndexedEntry::new)
            .collect();
        Some(Self { entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&CatalogEntry> {
        self.entries
            .iter()
            .map(|indexed| &indexed.entry)
            .find(|entry| entry.id == id)
    }

    /// Entries matching `query`, in upstream order, one page at a time.
    ///
    /// The cursor is the offset into the filtered list, so it stays stable
    /// for as long as the index is not replaced.
    pub fn search(&self, query: &CatalogQuery<'_>) -> Result<CatalogPage<'_>, InvalidCursor> {
        let offset = match query.cursor {
            Some(cursor) => cursor.parse::<usize>().map_err(|_| InvalidCursor)?,
            None => 0,
        };
        let limit = query
            .limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE);
        let q = query.q.map(|q| fold(q.trim())).unwrap_or_default();
        let category = query.category.map(fold);

        let mut matching = self
            .entries
            .iter()
            .filter(|indexed| indexed.matches(&q, category.as_deref()))
            .skip(offset);
        let items: Vec<&CatalogEntry> = matching
            .by_ref()
            .take(limit)
            .map(|indexed| &indexed.entry)
            .collect();
        let next_cursor = matching.next().map(|_| (offset + items.len()).to_string());
        Ok(CatalogPage { items, next_cursor })
    }
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

    // -- CatalogIndex --------------------------------------------------------

    fn fixture_index() -> CatalogIndex {
        let doc: Value = serde_json::from_str(FIXTURE).unwrap();
        CatalogIndex::from_document(&doc).unwrap()
    }

    fn ids<'a>(page: &CatalogPage<'a>) -> Vec<&'a str> {
        page.items.iter().map(|e| e.id.as_str()).collect()
    }

    /// `n` installable entries named "Entry 0".."Entry n-1", all in category `c`.
    fn index_of(n: usize) -> CatalogIndex {
        let data: Vec<Value> = (0..n)
            .map(|i| {
                json!({
                    "id": format!("e/{i}"), "kind": "mcp", "name": format!("Entry {i}"),
                    "connectUrl": "https://e.example/mcp", "categories": ["c"]
                })
            })
            .collect();
        CatalogIndex::from_document(&json!({"version": 1, "data": data})).unwrap()
    }

    #[test]
    fn from_document_indexes_only_normalizable_entries() {
        assert_eq!(fixture_index().len(), 13);
    }

    #[test]
    fn from_document_rejects_things_that_are_not_catalogs() {
        for raw in [
            json!({"version": 1}),
            json!({"data": "nope"}),
            json!([1, 2]),
            Value::Null,
        ] {
            assert!(CatalogIndex::from_document(&raw).is_none(), "{raw}");
        }
    }

    #[test]
    fn from_document_accepts_an_empty_catalog() {
        let index = CatalogIndex::from_document(&json!({"data": []})).unwrap();

        assert!(index.is_empty());
    }

    #[test]
    fn empty_index_searches_to_an_empty_page() {
        let index = CatalogIndex::empty();

        let page = index.search(&CatalogQuery::default()).unwrap();

        assert!(page.items.is_empty());
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn get_finds_an_entry_by_id() {
        let index = fixture_index();

        assert_eq!(
            index.get("curated/linear-app-mcp").map(|e| e.name.as_str()),
            Some("Linear")
        );
        assert!(index.get("curated/github-com-cli").is_none());
        assert!(index.get("nope").is_none());
    }

    #[test]
    fn text_search_is_case_insensitive_over_name_domain_and_description() {
        let index = fixture_index();

        let by_name = index
            .search(&CatalogQuery {
                q: Some("LINEAR"),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(ids(&by_name), ["curated/linear-app-mcp"]);

        let by_domain = index
            .search(&CatalogQuery {
                q: Some("notion.com"),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(ids(&by_domain), ["curated/notion-com-mcp"]);

        let by_description = index
            .search(&CatalogQuery {
                q: Some("hosted mcp server exposes query, monitor"),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(ids(&by_description), ["curated/axiom-co-mcp"]);
    }

    #[test]
    fn blank_text_search_matches_everything() {
        let index = fixture_index();

        let page = index
            .search(&CatalogQuery {
                q: Some("   "),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(page.items.len(), 13);
    }

    #[test]
    fn category_filter_is_case_insensitive_and_exact() {
        let index = fixture_index();

        let page = index
            .search(&CatalogQuery {
                category: Some("Observability"),
                ..Default::default()
            })
            .unwrap();
        let mut found = ids(&page);
        found.sort();
        assert_eq!(found, ["curated/axiom-co-mcp", "curated/sentry-io-mcp"]);

        let partial = index
            .search(&CatalogQuery {
                category: Some("observ"),
                ..Default::default()
            })
            .unwrap();
        assert!(partial.items.is_empty());
    }

    #[test]
    fn text_and_category_filters_combine() {
        let index = fixture_index();

        let page = index
            .search(&CatalogQuery {
                q: Some("sentry"),
                category: Some("observability"),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(ids(&page), ["curated/sentry-io-mcp"]);

        let none = index
            .search(&CatalogQuery {
                q: Some("linear"),
                category: Some("observability"),
                ..Default::default()
            })
            .unwrap();
        assert!(none.items.is_empty());
    }

    #[test]
    fn default_page_size_is_fifty() {
        let index = index_of(120);

        let page = index.search(&CatalogQuery::default()).unwrap();

        assert_eq!(page.items.len(), 50);
        assert!(page.next_cursor.is_some());
    }

    #[test]
    fn limit_is_capped_at_two_hundred_and_at_least_one() {
        let index = index_of(250);

        let big = index
            .search(&CatalogQuery {
                limit: Some(10_000),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(big.items.len(), 200);

        let zero = index
            .search(&CatalogQuery {
                limit: Some(0),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(zero.items.len(), 1);
    }

    #[test]
    fn cursor_pages_through_every_entry_once_in_order() {
        let index = index_of(7);
        let mut seen = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let page = index
                .search(&CatalogQuery {
                    limit: Some(3),
                    cursor: cursor.as_deref(),
                    ..Default::default()
                })
                .unwrap();
            seen.extend(page.items.iter().map(|e| e.id.clone()));
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        let expected: Vec<String> = (0..7).map(|i| format!("e/{i}")).collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn exact_fit_last_page_has_no_next_cursor() {
        let index = index_of(6);

        let first = index
            .search(&CatalogQuery {
                limit: Some(3),
                ..Default::default()
            })
            .unwrap();
        let second = index
            .search(&CatalogQuery {
                limit: Some(3),
                cursor: first.next_cursor.as_deref(),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(second.items.len(), 3);
        assert_eq!(second.next_cursor, None);
    }

    #[test]
    fn invalid_cursor_is_rejected() {
        let index = index_of(3);

        for cursor in ["abc", "-1", "", "1.5"] {
            let result = index.search(&CatalogQuery {
                cursor: Some(cursor),
                ..Default::default()
            });
            assert_eq!(result.err(), Some(InvalidCursor), "cursor {cursor:?}");
        }
    }

    #[test]
    fn cursor_past_the_end_yields_an_empty_last_page() {
        let index = index_of(3);

        let page = index
            .search(&CatalogQuery {
                cursor: Some("99"),
                ..Default::default()
            })
            .unwrap();

        assert!(page.items.is_empty());
        assert_eq!(page.next_cursor, None);
    }

    // -- Review follow-ups ---------------------------------------------------

    #[test]
    fn https_connect_urls_without_a_host_are_skipped() {
        for url in [
            "https://?x",
            "https://",
            "https://#frag",
            "https://:443/mcp",
        ] {
            let raw = json!({
                "id": "x/y", "kind": "mcp", "name": "Y", "connectUrl": url
            });

            assert_eq!(normalize(&raw), None, "{url:?} has no host");
        }
    }

    #[test]
    fn header_names_must_be_http_tokens() {
        for template in [
            "Bad(Name): {a}",
            "Bad,Name: {a}",
            "Bad/Name: {a}",
            "Bäd: {a}",
        ] {
            let raw = mcp_with_auth(json!({"kind": "api_key", "header": template}));

            let auth = normalize(&raw).unwrap().auth;

            assert_eq!(auth.kind, CatalogAuthKind::Header, "template {template:?}");
            assert_eq!(auth.header_name, None, "template {template:?}");
        }
        let raw = mcp_with_auth(json!({"kind": "api_key", "header": "X-Api_Key.1: {a}"}));
        assert_eq!(
            normalize(&raw).unwrap().auth.header_name.as_deref(),
            Some("X-Api_Key.1")
        );
    }

    fn named_index(name: &str, category: &str) -> CatalogIndex {
        let doc = json!({"data": [{
            "id": "u/1", "kind": "mcp", "name": name,
            "connectUrl": "https://u.example/mcp", "categories": [category]
        }]});
        CatalogIndex::from_document(&doc).unwrap()
    }

    #[test]
    fn text_search_ignores_case_for_final_sigma() {
        let index = named_index("ΟΣΑ", "c");

        for q in ["ΟΣ", "οσ", "ΟΣΑ", "οσα"] {
            let page = index
                .search(&CatalogQuery {
                    q: Some(q),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(page.items.len(), 1, "query {q:?}");
        }
    }

    #[test]
    fn category_filter_ignores_case_for_non_ascii_names() {
        let index = named_index("Cours", "Éducation");

        for category in ["Éducation", "éducation", "ÉDUCATION"] {
            let page = index
                .search(&CatalogQuery {
                    category: Some(category),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(page.items.len(), 1, "category {category:?}");
        }
    }

    #[test]
    fn urls_with_credentials_or_fragments_are_skipped() {
        for url in [
            "https://bob:token@y.example/mcp",
            "https://bob@y.example/mcp",
            "https://y.example/mcp#frag",
        ] {
            let raw = json!({
                "id": "x/y", "kind": "mcp", "name": "Y",
                "connectUrl": url, "auth": {"kind": "none"}
            });
            assert_eq!(normalize(&raw), None, "{url}");
        }
    }
}
