//! Structural validation of surfaces: limits, structure, regex safety and
//! the URL policy.
//!
//! [`validate`] is pure and synchronous. The async caller resolves attachment
//! ownership first (see [`SurfaceDraft::attachment_ids`]) and passes it in.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::Value;
use uuid::Uuid;

use crate::catalog::{Catalog, ValidationIssue};
use crate::messages::{Component, DataUpdate, SurfaceDraft, SurfacePatch};
use crate::safety::{attachment_id, check_image_url, check_link_url, regex_problem};

const MAX_COMPONENTS: usize = 100;
const MAX_DEPTH: usize = 12;
const MAX_SURFACE_BYTES: usize = 64 * 1024;
const MAX_STRING_BYTES: usize = 4 * 1024;
const MAX_FALLBACK_CHARS: usize = 2000;
const KNOWN_DIRECTIVES: [&str; 3] = ["@path", "@call", "@checks"];

/// Data the async caller resolves before validation.
#[derive(Debug, Clone, Default)]
pub struct UrlPolicy {
    /// Attachments of the surface's conversation that `Image` may reference.
    pub allowed_attachment_ids: HashSet<Uuid>,
    /// `[ui] image_hosts`; `None` or empty allows any `https:` host.
    pub image_hosts: Option<Vec<String>>,
}

/// A surface that passed [`validate`]; only `validate` and `merge` build one.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidSurface {
    draft: SurfaceDraft,
}

impl ValidSurface {
    pub fn draft(&self) -> &SurfaceDraft {
        &self.draft
    }

    pub fn into_draft(self) -> SurfaceDraft {
        self.draft
    }

    /// Apply `patch` (components upserted by id, `data_updates` applied, `null`
    /// deleting) and validate the merged surface.
    ///
    /// `policy` must also allow the attachments the surface already uses, so
    /// the caller unions the ids of [`Self::draft`] and the patch.
    pub fn merge(
        &self,
        patch: &SurfacePatch,
        policy: &UrlPolicy,
    ) -> Result<ValidSurface, Vec<ValidationIssue>> {
        let mut draft = self.draft.clone();
        if let Some(title) = &patch.title {
            draft.title = title.clone();
        }
        if let Some(text) = &patch.fallback_text {
            draft.fallback_text = text.clone();
        }
        for c in &patch.components {
            match draft.components.iter_mut().find(|x| x.id == c.id) {
                Some(slot) => *slot = c.clone(),
                None => draft.components.push(c.clone()),
            }
        }
        let mut issues = Vec::new();
        for (i, u) in patch.data_updates.iter().enumerate() {
            if let Err(msg) = apply_data_update(&mut draft.data_model, u) {
                issues.push(ValidationIssue::new(
                    "DATA_PATH",
                    format!("/data_updates/{i}/path"),
                    msg,
                ));
            }
        }
        if !issues.is_empty() {
            return Err(issues);
        }
        validate(&draft, policy)
    }

    /// The `event.name` of every Button action, for the action routes.
    pub fn declared_action_names(&self) -> BTreeSet<String> {
        self.draft.declared_action_names()
    }
}

impl SurfaceDraft {
    /// The `event.name` of every Button action.
    pub fn declared_action_names(&self) -> BTreeSet<String> {
        self.components
            .iter()
            .filter(|c| c.component == "Button")
            .filter_map(|c| c.props.get("action")?.pointer("/event/name")?.as_str())
            .map(str::to_owned)
            .collect()
    }

    /// Attachment ids the draft references, for the caller's ownership lookup.
    pub fn attachment_ids(&self) -> HashSet<Uuid> {
        let mut ids = literal_attachment_ids(&self.components);
        collect_ids(self.data_model.iter(), &mut ids);
        ids
    }
}

impl SurfacePatch {
    /// Attachment ids the patch references, for the caller's ownership lookup.
    pub fn attachment_ids(&self) -> HashSet<Uuid> {
        let mut ids = literal_attachment_ids(&self.components);
        collect_ids(self.data_updates.iter().map(|u| &u.value), &mut ids);
        ids
    }
}

/// Validate a draft against the catalog, the limits and the URL policy,
/// returning every issue at once.
pub fn validate(
    draft: &SurfaceDraft,
    policy: &UrlPolicy,
) -> Result<ValidSurface, Vec<ValidationIssue>> {
    let catalog = Catalog::embedded()
        .map_err(|e| vec![ValidationIssue::new("CATALOG", "", format!("{e:#}"))])?;
    let mut issues = Vec::new();

    let fallback_len = draft.fallback_text.chars().count();
    if !(1..=MAX_FALLBACK_CHARS).contains(&fallback_len) {
        issues.push(ValidationIssue::new(
            "FALLBACK_TEXT",
            "/fallback_text",
            format!(
                "fallback_text must be 1 to {MAX_FALLBACK_CHARS} characters, got {fallback_len}"
            ),
        ));
    }
    let size = serde_json::to_vec(draft).map_or(usize::MAX, |b| b.len());
    if size > MAX_SURFACE_BYTES {
        issues.push(ValidationIssue::new(
            "TOO_LARGE",
            "",
            format!("surface is {size} bytes, the limit is {MAX_SURFACE_BYTES}"),
        ));
        // Walking an oversized surface is wasted work.
        return Err(issues);
    }
    if draft.components.len() > MAX_COMPONENTS {
        issues.push(ValidationIssue::new(
            "TOO_MANY_COMPONENTS",
            "/components",
            format!(
                "{} components, the limit is {MAX_COMPONENTS}",
                draft.components.len()
            ),
        ));
    }

    let mut scan = Scan::default();
    for (i, c) in draft.components.iter().enumerate() {
        let value = c.to_value();
        let at = format!("/components/{i}");
        for issue in catalog.validate_component(&value) {
            let path = if issue.code == "UNKNOWN_COMPONENT" {
                at.clone()
            } else {
                format!("{at}{}", issue.path)
            };
            issues.push(ValidationIssue { path, ..issue });
        }
        walk(&value, &at, &mut |v, path| scan.visit(v, path));
    }
    if let Some(model) = &draft.data_model {
        walk(model, "/data_model", &mut |v, path| {
            scan.check_string(v, path)
        });
    }
    issues.append(&mut scan.issues);
    check_structure(&draft.components, &mut issues);
    for (path, pattern) in &scan.regexes {
        if let Some(why) = regex_problem(pattern) {
            issues.push(ValidationIssue::new("UNSAFE_REGEX", path, why));
        }
    }
    check_urls(&scan, draft.data_model.as_ref(), policy, &mut issues);

    if issues.is_empty() {
        Ok(ValidSurface {
            draft: draft.clone(),
        })
    } else {
        Err(issues)
    }
}

// -- Structure --

fn check_structure(components: &[Component], issues: &mut Vec<ValidationIssue>) {
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (i, c) in components.iter().enumerate() {
        if index.insert(&c.id, i).is_some() {
            issues.push(ValidationIssue::new(
                "DUPLICATE_ID",
                format!("/components/{i}/id"),
                format!("component id \"{}\" is used more than once", c.id),
            ));
        }
    }
    let Some(&root) = index.get("root") else {
        issues.push(ValidationIssue::new(
            "MISSING_ROOT",
            "/components",
            "exactly one component must have the id \"root\"",
        ));
        return;
    };
    let children: Vec<Vec<(String, String)>> = components.iter().map(child_refs).collect();
    for (i, refs) in children.iter().enumerate() {
        for (path, id) in refs {
            if !index.contains_key(id.as_str()) {
                issues.push(ValidationIssue::new(
                    "DANGLING_CHILD",
                    format!("/components/{i}{path}"),
                    format!("child \"{id}\" does not exist"),
                ));
            }
        }
    }
    let mut heights = HashMap::new();
    let mut on_path = HashSet::new();
    match height(root, &children, &index, &mut heights, &mut on_path) {
        None => issues.push(ValidationIssue::new(
            "CYCLE",
            "/components",
            "the component tree contains a cycle",
        )),
        Some(h) if h > MAX_DEPTH => issues.push(ValidationIssue::new(
            "TOO_DEEP",
            "/components",
            format!("the tree is {h} levels deep, the limit is {MAX_DEPTH}"),
        )),
        Some(_) => {}
    }
}

/// Levels from `node` down (a leaf is 1), memoized so shared children cost
/// once; `None` when a cycle is reachable.
fn height(
    node: usize,
    children: &[Vec<(String, String)>],
    index: &HashMap<&str, usize>,
    heights: &mut HashMap<usize, Option<usize>>,
    on_path: &mut HashSet<usize>,
) -> Option<usize> {
    if let Some(h) = heights.get(&node) {
        return *h;
    }
    if !on_path.insert(node) {
        return None;
    }
    let mut deepest = 0;
    let mut acyclic = true;
    for (_, id) in &children[node] {
        let Some(&next) = index.get(id.as_str()) else {
            continue;
        };
        match height(next, children, index, heights, on_path) {
            Some(h) => deepest = deepest.max(h),
            None => acyclic = false,
        }
    }
    on_path.remove(&node);
    let result = acyclic.then_some(deepest + 1);
    heights.insert(node, result);
    result
}

/// Child references of one component as `(pointer suffix, id)`.
fn child_refs(c: &Component) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut push = |path: String, v: Option<&Value>| {
        if let Some(id) = v.and_then(Value::as_str) {
            out.push((path, id.to_owned()));
        }
    };
    push("/child".to_owned(), c.props.get("child"));
    match c.props.get("children") {
        Some(Value::Array(ids)) => {
            for (i, id) in ids.iter().enumerate() {
                push(format!("/children/{i}"), Some(id));
            }
        }
        Some(template) => push(
            "/children/componentId".to_owned(),
            template.get("componentId"),
        ),
        None => {}
    }
    if let Some(Value::Array(tabs)) = c.props.get("tabs") {
        for (i, tab) in tabs.iter().enumerate() {
            push(format!("/tabs/{i}/child"), tab.get("child"));
        }
    }
    out
}

// -- Scan: strings, directives, regexes and URL targets --

#[derive(Default)]
struct Scan {
    issues: Vec<ValidationIssue>,
    regexes: Vec<(String, String)>,
    image_urls: Vec<UrlRef>,
    link_urls: Vec<UrlRef>,
}

/// A URL position: a literal, or a binding to a data-model pointer.
struct UrlRef {
    path: String,
    target: UrlTarget,
}

enum UrlTarget {
    Literal(String),
    Bound(String),
}

impl Scan {
    fn visit(&mut self, v: &Value, path: &str) {
        self.check_string(v, path);
        let Value::Object(map) = v else { return };
        for key in map.keys().filter(|k| k.starts_with('@')) {
            if !KNOWN_DIRECTIVES.contains(&key.as_str()) {
                self.issues.push(ValidationIssue::new(
                    "UNKNOWN_DIRECTIVE",
                    format!("{path}/{}", escape(key)),
                    format!("unknown directive \"{key}\""),
                ));
            }
        }
        if map.get("component").and_then(Value::as_str) == Some("Image") {
            self.image_urls.extend(url_ref(map.get("url"), path, "url"));
        }
        let args = map.get("args");
        match map.get("@call").and_then(Value::as_str) {
            Some("openUrl") => {
                self.link_urls
                    .extend(url_ref(args.and_then(|a| a.get("url")), path, "args/url"))
            }
            Some("regex") => {
                if let Some(p) = args.and_then(|a| a.get("pattern")).and_then(Value::as_str) {
                    self.regexes
                        .push((format!("{path}/args/pattern"), p.to_owned()));
                }
            }
            _ => {}
        }
    }

    fn check_string(&mut self, v: &Value, path: &str) {
        if let Value::String(s) = v
            && s.len() > MAX_STRING_BYTES
        {
            self.issues.push(ValidationIssue::new(
                "STRING_TOO_LONG",
                path,
                format!(
                    "string is {} bytes, the limit is {MAX_STRING_BYTES}",
                    s.len()
                ),
            ));
        }
    }
}

fn url_ref(v: Option<&Value>, parent: &str, key: &str) -> Option<UrlRef> {
    let path = format!("{parent}/{key}");
    let target = match v? {
        Value::String(s) => UrlTarget::Literal(s.clone()),
        Value::Object(o) => UrlTarget::Bound(o.get("@path")?.as_str()?.to_owned()),
        _ => return None,
    };
    Some(UrlRef { path, target })
}

fn check_urls(
    scan: &Scan,
    data_model: Option<&Value>,
    policy: &UrlPolicy,
    issues: &mut Vec<ValidationIssue>,
) {
    check_refs(&scan.image_urls, data_model, issues, |u| {
        check_image_url(u, policy)
    });
    check_refs(&scan.link_urls, data_model, issues, check_link_url);
}

/// Check each literal, and each bound initial value, with `check`.
fn check_refs(
    refs: &[UrlRef],
    data_model: Option<&Value>,
    issues: &mut Vec<ValidationIssue>,
    check: impl Fn(&str) -> Result<(), String>,
) {
    for r in refs {
        let (url, path) = match &r.target {
            UrlTarget::Literal(u) => (Some(u.as_str()), r.path.clone()),
            UrlTarget::Bound(ptr) => (
                data_model
                    .and_then(|m| m.pointer(ptr))
                    .and_then(Value::as_str),
                format!("/data_model{ptr}"),
            ),
        };
        if let Some(Err(why)) = url.map(&check) {
            issues.push(ValidationIssue::new("URL_POLICY", path, why));
        }
    }
}

// -- Helpers --

/// Visit `v` and every node below it with its JSON Pointer.
fn walk(v: &Value, path: &str, f: &mut dyn FnMut(&Value, &str)) {
    f(v, path);
    match v {
        Value::Object(m) => m
            .iter()
            .for_each(|(k, c)| walk(c, &format!("{path}/{}", escape(k)), f)),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .for_each(|(i, c)| walk(c, &format!("{path}/{i}"), f)),
        _ => {}
    }
}

fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn literal_attachment_ids(components: &[Component]) -> HashSet<Uuid> {
    components
        .iter()
        .filter(|c| c.component == "Image")
        .filter_map(|c| attachment_id(c.props.get("url")?.as_str()?))
        .collect()
}

/// Over-approximates: any string at all in `values` that is an attachment path.
fn collect_ids<'a>(values: impl Iterator<Item = &'a Value>, ids: &mut HashSet<Uuid>) {
    for v in values {
        walk(v, "", &mut |n, _| {
            if let Some(id) = n.as_str().and_then(attachment_id) {
                ids.insert(id);
            }
        });
    }
}

/// Set or (for `null`) delete the value at a JSON Pointer in the data model.
fn apply_data_update(model: &mut Option<Value>, u: &DataUpdate) -> Result<(), String> {
    let tokens: Vec<String> = match u.path.as_str() {
        "" | "/" => Vec::new(),
        p => p
            .strip_prefix('/')
            .ok_or("path must be a JSON Pointer starting with /")?
            .split('/')
            .map(|t| t.replace("~1", "/").replace("~0", "~"))
            .collect(),
    };
    let Some((last, parents)) = tokens.split_last() else {
        *model = (!u.value.is_null()).then(|| u.value.clone());
        return Ok(());
    };
    let mut node = model.get_or_insert_with(|| Value::Object(Default::default()));
    for t in parents {
        node = match node {
            Value::Object(m) => m
                .entry(t.clone())
                .or_insert_with(|| Value::Object(Default::default())),
            Value::Array(a) => t
                .parse::<usize>()
                .ok()
                .and_then(|i| a.get_mut(i))
                .ok_or_else(|| format!("no array element at \"{t}\""))?,
            _ => return Err(format!("cannot descend into a scalar at \"{t}\"")),
        };
    }
    match node {
        Value::Object(m) if u.value.is_null() => {
            m.remove(last);
        }
        Value::Object(m) => {
            m.insert(last.clone(), u.value.clone());
        }
        Value::Array(a) => {
            let i: usize = last
                .parse()
                .map_err(|_| format!("\"{last}\" is not an array index"))?;
            match (i.cmp(&a.len()), u.value.is_null()) {
                (std::cmp::Ordering::Less, true) => {
                    a.remove(i);
                }
                (std::cmp::Ordering::Less, false) => a[i] = u.value.clone(),
                (std::cmp::Ordering::Equal, false) => a.push(u.value.clone()),
                _ => return Err(format!("array index {i} is out of range")),
            }
        }
        _ => return Err(format!("cannot set \"{last}\" on a scalar")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::messages::{Component, DataUpdate};

    fn comp(v: Value) -> Component {
        serde_json::from_value(v).expect("component")
    }

    fn text(id: &str, s: &str) -> Component {
        comp(json!({"id": id, "component": "Text", "text": s}))
    }

    fn column(id: &str, children: &[&str]) -> Component {
        comp(json!({"id": id, "component": "Column", "children": children}))
    }

    fn draft(components: Vec<Component>) -> SurfaceDraft {
        SurfaceDraft {
            title: "Title".to_owned(),
            fallback_text: "Fallback".to_owned(),
            components,
            data_model: None,
            send_data_model: true,
        }
    }

    fn simple() -> SurfaceDraft {
        draft(vec![column("root", &["t"]), text("t", "hi")])
    }

    fn issues(d: &SurfaceDraft, p: &UrlPolicy) -> Vec<ValidationIssue> {
        validate(d, p).expect_err("expected validation to fail")
    }

    fn has(list: &[ValidationIssue], code: &str, path_prefix: &str) -> bool {
        list.iter()
            .any(|i| i.code == code && i.path.starts_with(path_prefix))
    }

    fn chain(len: usize) -> SurfaceDraft {
        let mut cs = Vec::new();
        for i in 0..len {
            let id = if i == 0 {
                "root".to_owned()
            } else {
                format!("n{i}")
            };
            if i + 1 < len {
                cs.push(column(&id, &[&format!("n{}", i + 1)]));
            } else {
                cs.push(text(&id, "leaf"));
            }
        }
        draft(cs)
    }

    fn image(id: &str, url: Value) -> Component {
        comp(json!({"id": id, "component": "Image", "url": url}))
    }

    fn with_children(root_children: &[&str], rest: Vec<Component>) -> SurfaceDraft {
        let mut cs = vec![column("root", root_children)];
        cs.extend(rest);
        draft(cs)
    }

    // -- structure and limits --

    #[test]
    fn valid_surface_passes() {
        let ok = validate(&simple(), &UrlPolicy::default()).expect("valid");
        assert_eq!(ok.draft().components.len(), 2);
    }

    #[test]
    fn unknown_component_reports_its_index() {
        let d = with_children(
            &["x"],
            vec![comp(json!({"id": "x", "component": "Script"}))],
        );
        let list = issues(&d, &UrlPolicy::default());
        assert!(has(&list, "UNKNOWN_COMPONENT", "/components/1"), "{list:?}");
    }

    #[test]
    fn dangling_child_names_the_missing_id() {
        let d = draft(vec![column("root", &["ghost"])]);
        let list = issues(&d, &UrlPolicy::default());
        let i = list
            .iter()
            .find(|i| i.code == "DANGLING_CHILD")
            .expect("issue");
        assert!(i.message.contains("ghost"), "{i:?}");
        assert!(i.path.starts_with("/components/0"), "{i:?}");
    }

    #[test]
    fn duplicate_and_extra_root_ids_fail() {
        let d = draft(vec![column("root", &["t"]), text("t", "a"), text("t", "b")]);
        assert!(has(
            &issues(&d, &UrlPolicy::default()),
            "DUPLICATE_ID",
            "/components/2"
        ));
        let d = draft(vec![text("root", "a"), text("root", "b")]);
        assert!(has(
            &issues(&d, &UrlPolicy::default()),
            "DUPLICATE_ID",
            "/components/1"
        ));
    }

    #[test]
    fn missing_root_fails() {
        let d = draft(vec![text("t", "a")]);
        assert!(has(
            &issues(&d, &UrlPolicy::default()),
            "MISSING_ROOT",
            "/components"
        ));
    }

    #[test]
    fn cycle_fails() {
        let d = draft(vec![column("root", &["a"]), column("a", &["root"])]);
        assert!(has(
            &issues(&d, &UrlPolicy::default()),
            "CYCLE",
            "/components"
        ));
    }

    #[test]
    fn depth_twelve_passes_and_thirteen_fails() {
        assert!(validate(&chain(12), &UrlPolicy::default()).is_ok());
        let list = issues(&chain(13), &UrlPolicy::default());
        assert!(has(&list, "TOO_DEEP", "/components"), "{list:?}");
    }

    #[test]
    fn shared_children_do_not_blow_up() {
        // 12 levels, each fanning out to the next level twice: 2^12 paths.
        let mut cs = Vec::new();
        for i in 0..11 {
            let id = if i == 0 {
                "root".to_owned()
            } else {
                format!("n{i}")
            };
            let next = format!("n{}", i + 1);
            cs.push(column(&id, &[&next, &next]));
        }
        cs.push(text("n11", "leaf"));
        assert!(validate(&draft(cs), &UrlPolicy::default()).is_ok());
    }

    #[test]
    fn component_count_limit_is_100() {
        let kids: Vec<String> = (0..99).map(|i| format!("t{i}")).collect();
        let refs: Vec<&str> = kids.iter().map(String::as_str).collect();
        let mut cs = vec![column("root", &refs)];
        cs.extend(kids.iter().map(|k| text(k, "x")));
        assert_eq!(cs.len(), 100);
        assert!(validate(&draft(cs.clone()), &UrlPolicy::default()).is_ok());
        cs.push(text("extra", "x"));
        let list = issues(&draft(cs), &UrlPolicy::default());
        assert!(has(&list, "TOO_MANY_COMPONENTS", "/components"), "{list:?}");
    }

    #[test]
    fn size_limit_is_64_kib() {
        let kids: Vec<String> = (0..20).map(|i| format!("t{i}")).collect();
        let refs: Vec<&str> = kids.iter().map(String::as_str).collect();
        let mut cs = vec![column("root", &refs)];
        cs.extend(kids.iter().map(|k| text(k, &"x".repeat(4000))));
        let list = issues(&draft(cs), &UrlPolicy::default());
        assert!(list.iter().any(|i| i.code == "TOO_LARGE"), "{list:?}");
    }

    #[test]
    fn string_literal_limit_is_4_kib() {
        let ok = with_children(&["t"], vec![text("t", &"x".repeat(4096))]);
        assert!(validate(&ok, &UrlPolicy::default()).is_ok());
        let bad = with_children(&["t"], vec![text("t", &"x".repeat(4097))]);
        let list = issues(&bad, &UrlPolicy::default());
        assert!(
            has(&list, "STRING_TOO_LONG", "/components/1/text"),
            "{list:?}"
        );
    }

    #[test]
    fn unknown_at_key_is_rejected() {
        let d = with_children(
            &["f"],
            vec![comp(json!({
                "id": "f", "component": "TextField", "label": "L", "value": "v",
                "@bogus": []
            }))],
        );
        let list = issues(&d, &UrlPolicy::default());
        assert!(
            has(&list, "UNKNOWN_DIRECTIVE", "/components/1/@bogus"),
            "{list:?}"
        );
    }

    #[test]
    fn multiple_issues_are_reported_together() {
        let d = draft(vec![text("t", &"x".repeat(5000))]);
        let list = issues(&d, &UrlPolicy::default());
        assert!(list.iter().any(|i| i.code == "MISSING_ROOT"), "{list:?}");
        assert!(list.iter().any(|i| i.code == "STRING_TOO_LONG"), "{list:?}");
    }

    #[test]
    fn fallback_text_must_be_1_to_2000_chars() {
        for (len, ok) in [(0, false), (1, true), (2000, true), (2001, false)] {
            let mut d = simple();
            d.fallback_text = "é".repeat(len);
            let r = validate(&d, &UrlPolicy::default());
            assert_eq!(r.is_ok(), ok, "len {len}");
            if let Err(list) = r {
                assert!(has(&list, "FALLBACK_TEXT", "/fallback_text"), "{list:?}");
            }
        }
    }

    // -- regex --

    fn field_with_regex(pattern: &str) -> SurfaceDraft {
        with_children(
            &["f"],
            vec![comp(json!({
                "id": "f", "component": "TextField", "label": "Zip", "value": {"@path": "/zip"},
                "@checks": [{
                    "condition": {"@call": "regex", "args": {"value": {"@path": "/zip"}, "pattern": pattern}},
                    "message": "bad"
                }]
            }))],
        )
    }

    #[test]
    fn unsafe_regex_is_reported_at_the_pattern() {
        for p in ["(a+)+$", "(a|aa)+"] {
            let list = issues(&field_with_regex(p), &UrlPolicy::default());
            assert!(
                has(
                    &list,
                    "UNSAFE_REGEX",
                    "/components/1/@checks/0/condition/args/pattern"
                ),
                "{p}: {list:?}"
            );
        }
        assert!(validate(&field_with_regex("^[0-9]{5}$"), &UrlPolicy::default()).is_ok());
    }

    // -- URL policy --

    fn button_open(url: Value) -> SurfaceDraft {
        with_children(
            &["b"],
            vec![
                comp(json!({
                    "id": "b", "component": "Button", "child": "l",
                    "action": {"functionCall": {"@call": "openUrl", "args": {"url": url}}}
                })),
                text("l", "Open"),
            ],
        )
    }

    #[test]
    fn open_url_targets_follow_the_scheme_policy() {
        let p = UrlPolicy::default();
        let list = issues(&button_open(json!("javascript:alert(1)")), &p);
        assert!(
            has(
                &list,
                "URL_POLICY",
                "/components/1/action/functionCall/args/url"
            ),
            "{list:?}"
        );
        assert!(has(
            &issues(&button_open(json!("data:text/html,x")), &p),
            "URL_POLICY",
            "/components"
        ));
        assert!(has(
            &issues(&button_open(json!("http://example.com")), &p),
            "URL_POLICY",
            "/components"
        ));
        assert!(validate(&button_open(json!("mailto:a@example.com")), &p).is_ok());
        // The link allowlist is a client concern: any https host validates.
        assert!(validate(&button_open(json!("https://anywhere.example/x")), &p).is_ok());
    }

    #[test]
    fn image_urls_follow_ownership_and_host_policy() {
        let owned = Uuid::new_v4();
        let foreign = Uuid::new_v4();
        let p = UrlPolicy {
            allowed_attachment_ids: HashSet::from([owned]),
            image_hosts: None,
        };
        let img = |u: String| with_children(&["i"], vec![image("i", json!(u))]);
        assert!(validate(&img(format!("/api/attachments/{owned}")), &p).is_ok());
        assert!(has(
            &issues(&img(format!("/api/attachments/{foreign}")), &p),
            "URL_POLICY",
            "/components/1/url"
        ));
        assert!(validate(&img("https://any.example/p.png".to_owned()), &p).is_ok());

        let listed = UrlPolicy {
            image_hosts: Some(vec!["images.example.com".to_owned()]),
            ..UrlPolicy::default()
        };
        let list = issues(
            &img("https://tracker.example.net/p.png".to_owned()),
            &listed,
        );
        assert!(
            list.iter()
                .any(|i| i.code == "URL_POLICY" && i.message.contains("tracker.example.net"))
        );
        let empty = UrlPolicy {
            image_hosts: Some(vec![]),
            ..UrlPolicy::default()
        };
        assert!(validate(&img("https://any.example/p.png".to_owned()), &empty).is_ok());
    }

    #[test]
    fn bound_initial_values_are_checked() {
        let mut d = with_children(&["i"], vec![image("i", json!({"@path": "/img"}))]);
        d.data_model = Some(json!({"img": "data:image/png;base64,AAAA"}));
        let list = issues(&d, &UrlPolicy::default());
        assert!(has(&list, "URL_POLICY", "/data_model/img"), "{list:?}");
        d.data_model = Some(json!({"img": "https://ok.example/p.png"}));
        assert!(validate(&d, &UrlPolicy::default()).is_ok());

        let mut b = button_open(json!({"@path": "/go"}));
        b.data_model = Some(json!({"go": "javascript:alert(1)"}));
        assert!(has(
            &issues(&b, &UrlPolicy::default()),
            "URL_POLICY",
            "/data_model/go"
        ));
    }

    #[test]
    fn attachment_ids_are_collected_from_literals_and_data() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut d = with_children(
            &["i"],
            vec![image("i", json!(format!("/api/attachments/{a}")))],
        );
        d.data_model = Some(json!({"x": [format!("/api/attachments/{b}")]}));
        assert_eq!(d.attachment_ids(), HashSet::from([a, b]));
        let patch = SurfacePatch {
            components: vec![image("i", json!(format!("/api/attachments/{a}")))],
            ..SurfacePatch::default()
        };
        assert_eq!(patch.attachment_ids(), HashSet::from([a]));
    }

    // -- merge --

    #[test]
    fn merge_upserts_components_and_applies_data_updates() {
        let mut d = simple();
        d.data_model = Some(json!({"a": 1, "gone": true}));
        let base = validate(&d, &UrlPolicy::default()).expect("valid");
        let patch = SurfacePatch {
            title: Some("New".to_owned()),
            fallback_text: Some("New fallback".to_owned()),
            components: vec![text("t", "changed"), text("u", "added")],
            data_updates: vec![
                DataUpdate {
                    path: "/a".to_owned(),
                    value: json!(2),
                },
                DataUpdate {
                    path: "/gone".to_owned(),
                    value: Value::Null,
                },
                DataUpdate {
                    path: "/nested/x".to_owned(),
                    value: json!("y"),
                },
            ],
        };
        let merged = base.merge(&patch, &UrlPolicy::default()).expect("merge");
        let m = merged.draft();
        assert_eq!(m.title, "New");
        assert_eq!(m.fallback_text, "New fallback");
        assert_eq!(m.components.len(), 3);
        assert_eq!(m.components[1].props["text"], json!("changed"));
        assert_eq!(m.data_model, Some(json!({"a": 2, "nested": {"x": "y"}})));
    }

    #[test]
    fn merge_validates_the_merged_surface() {
        let base = validate(&simple(), &UrlPolicy::default()).expect("valid");
        let extra: Vec<Component> = (0..100).map(|i| text(&format!("e{i}"), "x")).collect();
        let patch = SurfacePatch {
            components: extra,
            ..SurfacePatch::default()
        };
        let list = base
            .merge(&patch, &UrlPolicy::default())
            .expect_err("101+ components");
        assert!(has(&list, "TOO_MANY_COMPONENTS", "/components"));

        let bad_url = SurfacePatch {
            data_updates: vec![DataUpdate {
                path: "/img".to_owned(),
                value: json!("javascript:x"),
            }],
            ..SurfacePatch::default()
        };
        let mut d = with_children(&["i"], vec![image("i", json!({"@path": "/img"}))]);
        d.data_model = Some(json!({"img": "https://ok.example/a.png"}));
        let base = validate(&d, &UrlPolicy::default()).expect("valid");
        let list = base
            .merge(&bad_url, &UrlPolicy::default())
            .expect_err("unsafe update");
        assert!(has(&list, "URL_POLICY", "/data_model/img"));

        let bad_path = SurfacePatch {
            data_updates: vec![DataUpdate {
                path: "no-slash".to_owned(),
                value: json!(1),
            }],
            ..SurfacePatch::default()
        };
        let list = base
            .merge(&bad_path, &UrlPolicy::default())
            .expect_err("bad pointer");
        assert!(has(&list, "DATA_PATH", "/data_updates/0/path"));
    }

    // -- declared actions --

    #[test]
    fn declared_action_names_collects_button_events() {
        let d = with_children(
            &["a", "b", "c"],
            vec![
                comp(
                    json!({"id": "a", "component": "Button", "child": "l", "action": {"event": {"name": "submit"}}}),
                ),
                comp(
                    json!({"id": "b", "component": "Button", "child": "l", "action": {"event": {"name": "cancel"}}}),
                ),
                comp(
                    json!({"id": "c", "component": "Button", "child": "l", "action": {"functionCall": {"@call": "openUrl", "args": {"url": "https://a.example"}}}}),
                ),
                text("l", "Go"),
            ],
        );
        let names = validate(&d, &UrlPolicy::default())
            .expect("valid")
            .declared_action_names();
        assert_eq!(
            names,
            BTreeSet::from(["submit".to_owned(), "cancel".to_owned()])
        );
    }
}
