//! Skill resources: `skills://list`, `skills://<name>` and
//! `skills://<name>/{scripts|references|assets}/<file>`.

use std::path::Path;

use rmcp::ErrorData;
use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, ReadResourceResult, Resource,
    ResourceContents, ResourceTemplate,
};
use serde_json::json;

use assistant_skills::{AuxFileCategory, SkillDef};
use assistant_storage::SkillCatalog;

const URI_PREFIX: &str = "skills://";
const LIST_URI: &str = "skills://list";
const CATEGORIES: [AuxFileCategory; 3] = [
    AuxFileCategory::Scripts,
    AuxFileCategory::References,
    AuxFileCategory::Assets,
];

pub async fn list(catalog: &dyn SkillCatalog) -> ListResourcesResult {
    let mut resources = vec![
        Resource::new(LIST_URI, "All Skills")
            .with_description("List of all registered assistant skills")
            .with_mime_type("application/json"),
    ];

    for skill in catalog.list().await {
        resources.push(
            Resource::new(format!("{URI_PREFIX}{}", skill.name), skill.name.clone())
                .with_description(skill.description.clone())
                .with_mime_type("text/markdown"),
        );

        for (category, rel_path) in skill.auxiliary_files() {
            let filename = rel_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            resources.push(
                Resource::new(
                    format!("{URI_PREFIX}{}/{}", skill.name, rel_path.display()),
                    filename,
                )
                .with_description(format!(
                    "{} file for {} skill",
                    category.dir_name(),
                    skill.name
                ))
                .with_mime_type(category.mime_type()),
            );
        }
    }

    ListResourcesResult::with_all_items(resources)
}

pub fn templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::with_all_items(vec![
        ResourceTemplate::new("skills://{name}/{category}/{+path}", "Skill file").with_description(
            "A file bundled with a skill; category is scripts, references or assets",
        ),
    ])
}

pub async fn read(catalog: &dyn SkillCatalog, uri: &str) -> Result<ReadResourceResult, ErrorData> {
    let Some(path) = uri.strip_prefix(URI_PREFIX) else {
        return Err(ErrorData::invalid_params(
            format!("Unknown resource URI: {uri}"),
            None,
        ));
    };

    let contents = match path.splitn(3, '/').collect::<Vec<_>>()[..] {
        ["list"] => ResourceContents::text(skill_index(catalog).await, uri)
            .with_mime_type("application/json"),
        [name] => skill_markdown(catalog, name, uri).await?,
        [name, category, file] => {
            let Some(category) = CATEGORIES.iter().find(|c| c.dir_name() == category) else {
                return Err(invalid_uri(uri));
            };
            auxiliary_file(catalog, name, category, file, uri).await?
        }
        _ => return Err(invalid_uri(uri)),
    };

    Ok(ReadResourceResult::new(vec![contents]))
}

fn invalid_uri(uri: &str) -> ErrorData {
    ErrorData::invalid_params(format!("Invalid resource URI: {uri}"), None)
}

fn not_found(what: impl std::fmt::Display) -> ErrorData {
    ErrorData::resource_not_found(format!("{what} not found"), None)
}

async fn get_skill(catalog: &dyn SkillCatalog, name: &str) -> Result<SkillDef, ErrorData> {
    catalog
        .get(name)
        .await
        .ok_or_else(|| not_found(format!("Skill '{name}'")))
}

async fn skill_index(catalog: &dyn SkillCatalog) -> String {
    let skills: Vec<_> = catalog
        .list()
        .await
        .iter()
        .map(|s| {
            json!({
                "name": s.name,
                "description": s.description,
                "source": s.source.to_string(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&json!({ "skills": skills })).unwrap_or_default()
}

async fn skill_markdown(
    catalog: &dyn SkillCatalog,
    name: &str,
    uri: &str,
) -> Result<ResourceContents, ErrorData> {
    let skill = get_skill(catalog, name).await?;
    let text = tokio::fs::read_to_string(skill.dir.join("SKILL.md"))
        .await
        .unwrap_or(skill.body);
    Ok(ResourceContents::text(text, uri).with_mime_type("text/markdown"))
}

async fn auxiliary_file(
    catalog: &dyn SkillCatalog,
    name: &str,
    category: &AuxFileCategory,
    file: &str,
    uri: &str,
) -> Result<ResourceContents, ErrorData> {
    let dir_name = category.dir_name();
    let skill = get_skill(catalog, name).await?;
    let path = skill.dir.join(dir_name).join(file);
    let text = read_file_within(&skill.dir, &path)
        .await
        .map_err(|_| not_found(format!("File '{dir_name}/{file}' in skill '{name}'")))?;
    Ok(ResourceContents::text(text, uri).with_mime_type(category.mime_type()))
}

/// Reads `path` only when it resolves inside `root`, so `..` segments and
/// symlinks in a resource URI cannot reach files outside the skill directory.
async fn read_file_within(root: &Path, path: &Path) -> std::io::Result<String> {
    let root = tokio::fs::canonicalize(root).await?;
    let resolved = tokio::fs::canonicalize(path).await?;
    if !resolved.starts_with(&root) {
        return Err(std::io::Error::from(std::io::ErrorKind::NotFound));
    }
    tokio::fs::read_to_string(resolved).await
}
