//! Skill prompts and argument completion.
//!
//! Every installed skill is a prompt taking one optional `input` argument.
//! Completion offers skill names for the `name` argument of a prompt reference
//! and of the skill resource template.

use rmcp::ErrorData;
use rmcp::model::{
    CompleteRequestParams, CompleteResult, CompletionInfo, GetPromptRequestParams, GetPromptResult,
    ListPromptsResult, Prompt, PromptArgument, PromptMessage, Role,
};

use assistant_storage::SkillCatalog;

use crate::resources::SKILL_TEMPLATE;

const INPUT: &str = "input";
const NAME: &str = "name";

pub async fn list(catalog: &dyn SkillCatalog) -> ListPromptsResult {
    let prompts = catalog
        .list()
        .await
        .into_iter()
        .map(|skill| {
            Prompt::new(
                skill.name,
                Some(skill.description),
                Some(vec![
                    PromptArgument::new(INPUT)
                        .with_description("What to apply the skill to")
                        .with_required(false),
                ]),
            )
        })
        .collect();
    ListPromptsResult::with_all_items(prompts)
}

pub async fn get(
    catalog: &dyn SkillCatalog,
    request: &GetPromptRequestParams,
) -> Result<GetPromptResult, ErrorData> {
    let Some(skill) = catalog.get(&request.name).await else {
        return Err(ErrorData::invalid_params(
            format!("Unknown prompt: {}", request.name),
            None,
        ));
    };
    let input = request
        .arguments
        .as_ref()
        .and_then(|arguments| arguments.get(INPUT))
        .and_then(|value| value.as_str());
    let text = match input {
        Some(input) => format!("{}\n\n{input}", skill.body),
        None => skill.body,
    };
    Ok(
        GetPromptResult::new(vec![PromptMessage::new_text(Role::User, text)])
            .with_description(skill.description),
    )
}

pub async fn complete(
    catalog: &dyn SkillCatalog,
    request: &CompleteRequestParams,
) -> Result<CompleteResult, ErrorData> {
    let names_a_skill = request.r#ref.as_prompt_name().is_some()
        || request.r#ref.as_resource_uri() == Some(SKILL_TEMPLATE);
    if request.argument.name != NAME || !names_a_skill {
        return Ok(CompleteResult::default());
    }

    let prefix = request.argument.value.as_str();
    let mut names: Vec<String> = catalog
        .list()
        .await
        .into_iter()
        .map(|skill| skill.name)
        .filter(|name| name.starts_with(prefix))
        .collect();
    names.sort();
    let total = names.len();
    names.truncate(CompletionInfo::MAX_VALUES);
    let completion = CompletionInfo::with_pagination(
        names,
        u32::try_from(total).ok(),
        total > CompletionInfo::MAX_VALUES,
    )
    .map_err(|e| ErrorData::internal_error(e, None))?;
    Ok(CompleteResult::new(completion))
}
