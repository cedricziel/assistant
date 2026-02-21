//! Safety gate — evaluates whether a skill may be executed in the current
//! interface context.

use assistant_core::{Interface, SkillDef};

/// A stateless safety checker that enforces per-interface and per-skill rules.
pub struct SafetyGate;

impl SafetyGate {
    /// Check whether `skill` is allowed to run given the current `interface`
    /// and the operator-configured `disabled_skills` list.
    ///
    /// Returns `Ok(())` if the skill may proceed, or `Err(reason)` if it
    /// should be blocked.
    pub fn check(
        skill: &SkillDef,
        interface: &Interface,
        disabled_skills: &[String],
    ) -> Result<(), String> {
        // 1. Check the operator-level disabled list.
        if disabled_skills.iter().any(|s| s == &skill.name) {
            return Err(format!(
                "Skill '{}' is disabled by configuration",
                skill.name
            ));
        }

        // 2. Shell execution is not permitted over the Signal interface because
        //    it would allow arbitrary code execution triggered by external
        //    messages.
        if skill.name == "shell-exec" && *interface == Interface::Signal {
            return Err("shell-exec is not available via the Signal interface".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::skill::{SkillSource, SkillTier};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_skill(name: &str) -> SkillDef {
        SkillDef {
            name: name.to_string(),
            description: format!("Test skill: {name}"),
            license: None,
            compatibility: None,
            allowed_tools: vec![],
            metadata: HashMap::new(),
            body: String::new(),
            dir: PathBuf::from(format!("/tmp/{name}")),
            tier: SkillTier::Builtin,
            mutating: false,
            confirmation_required: false,
            source: SkillSource::Builtin,
        }
    }

    #[test]
    fn allowed_skill_passes() {
        let skill = make_skill("web-fetch");
        assert!(SafetyGate::check(&skill, &Interface::Cli, &[]).is_ok());
    }

    #[test]
    fn disabled_skill_is_blocked() {
        let skill = make_skill("web-fetch");
        let disabled = vec!["web-fetch".to_string()];
        let result = SafetyGate::check(&skill, &Interface::Cli, &disabled);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("disabled"));
    }

    #[test]
    fn shell_exec_blocked_on_signal() {
        let skill = make_skill("shell-exec");
        let result = SafetyGate::check(&skill, &Interface::Signal, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Signal"));
    }

    #[test]
    fn shell_exec_allowed_on_cli() {
        let skill = make_skill("shell-exec");
        assert!(SafetyGate::check(&skill, &Interface::Cli, &[]).is_ok());
    }

    #[test]
    fn shell_exec_blocked_before_disabled_check() {
        // Even if shell-exec is not in the disabled list it should be blocked
        // on Signal.
        let skill = make_skill("shell-exec");
        let result = SafetyGate::check(&skill, &Interface::Signal, &[]);
        assert!(result.is_err());
    }
}
