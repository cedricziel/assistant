//! Coverage tests for `assistant-skills` lifting both files past the
//! 80% floor. `parser.rs` already has unit tests; this file covers the
//! filesystem-walking branches (`parse_skill_dir`, `discover_skills`,
//! `embedded_builtin_skills`) and the SkillDef helpers in `skill.rs`.

use std::path::PathBuf;

use assistant_skills::parser::{
    discover_skills, embedded_builtin_skills, parse_skill_content, parse_skill_dir,
};
use assistant_skills::skill::{AuxFileCategory, SkillDef, SkillSource};
use tempfile::TempDir;

const VALID: &str = "---\nname: foo\ndescription: A foo skill\n---\nBody\n";

fn write_skill(root: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), body).unwrap();
    dir
}

// ── AuxFileCategory ─────────────────────────────────────────────────────────

#[test]
fn aux_file_category_dir_name() {
    assert_eq!(AuxFileCategory::Scripts.dir_name(), "scripts");
    assert_eq!(AuxFileCategory::References.dir_name(), "references");
    assert_eq!(AuxFileCategory::Assets.dir_name(), "assets");
}

#[test]
fn aux_file_category_mime_type() {
    assert_eq!(AuxFileCategory::Scripts.mime_type(), "text/plain");
    assert_eq!(AuxFileCategory::References.mime_type(), "text/markdown");
    assert_eq!(
        AuxFileCategory::Assets.mime_type(),
        "application/octet-stream"
    );
}

// ── SkillSource Display ────────────────────────────────────────────────────

#[test]
fn skill_source_display() {
    assert_eq!(SkillSource::Builtin.to_string(), "builtin");
    assert_eq!(SkillSource::User.to_string(), "user");
    assert_eq!(SkillSource::Project.to_string(), "project");
    assert_eq!(SkillSource::Installed.to_string(), "installed");
}

// ── SkillDef helpers ───────────────────────────────────────────────────────

#[test]
fn skill_def_is_builtin_true_for_builtin_source() {
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "foo", VALID);
    let skill = parse_skill_content(VALID, &dir, SkillSource::Builtin).unwrap();
    assert!(skill.is_builtin());
}

#[test]
fn skill_def_is_builtin_false_for_other_sources() {
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "foo", VALID);
    for source in [
        SkillSource::User,
        SkillSource::Project,
        SkillSource::Installed,
    ] {
        let skill = parse_skill_content(VALID, &dir, source).unwrap();
        assert!(!skill.is_builtin());
    }
}

#[test]
fn skill_def_auxiliary_files_returns_empty_when_no_subdirs() {
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "foo", VALID);
    let skill = parse_skill_content(VALID, &dir, SkillSource::User).unwrap();
    assert!(skill.auxiliary_files().is_empty());
    assert!(!skill.has_auxiliary_files());
}

#[test]
fn skill_def_auxiliary_files_lists_scripts_refs_assets() {
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "foo", VALID);
    std::fs::create_dir(dir.join("scripts")).unwrap();
    std::fs::create_dir(dir.join("references")).unwrap();
    std::fs::create_dir(dir.join("assets")).unwrap();
    std::fs::write(dir.join("scripts").join("run.sh"), "#!/bin/sh\n").unwrap();
    std::fs::write(dir.join("references").join("README.md"), "ref").unwrap();
    std::fs::write(dir.join("assets").join("logo.png"), &[0u8; 4]).unwrap();
    // Hidden files (dotfiles) should be skipped.
    std::fs::write(dir.join("scripts").join(".hidden"), "skip").unwrap();

    let skill = parse_skill_content(VALID, &dir, SkillSource::User).unwrap();
    let files = skill.auxiliary_files();
    assert_eq!(files.len(), 3, "got: {files:?}");
    assert!(skill.has_auxiliary_files());

    let categories: Vec<_> = files.iter().map(|(c, _)| c.clone()).collect();
    assert!(categories.contains(&AuxFileCategory::Scripts));
    assert!(categories.contains(&AuxFileCategory::References));
    assert!(categories.contains(&AuxFileCategory::Assets));
}

#[test]
fn skill_def_serializes_round_trip() {
    // Exercise the serde-derived path on SkillDef to keep the Serialize
    // impl from rotting under coverage tooling.
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "foo", VALID);
    let skill = parse_skill_content(VALID, &dir, SkillSource::User).unwrap();
    let json = serde_json::to_string(&skill).unwrap();
    let parsed: SkillDef = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, skill.name);
    assert_eq!(parsed.body, skill.body);
}

// ── parse_skill_dir ────────────────────────────────────────────────────────

#[test]
fn parse_skill_dir_reads_from_disk() {
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "foo", VALID);
    let skill = parse_skill_dir(&dir, SkillSource::User).unwrap();
    assert_eq!(skill.name, "foo");
    assert!(skill.body.contains("Body"));
}

#[test]
fn parse_skill_dir_missing_file_returns_error() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("nonexistent");
    let err = parse_skill_dir(&dir, SkillSource::User);
    assert!(err.is_err());
}

#[test]
fn parse_skill_dir_rejects_directory_name_mismatch() {
    let tmp = TempDir::new().unwrap();
    let dir = write_skill(tmp.path(), "actual-dir-name", VALID);
    // VALID has name=foo but dir is actual-dir-name
    let err = parse_skill_dir(&dir, SkillSource::User);
    assert!(err.is_err());
}

#[test]
fn parse_skill_dir_rejects_non_kebab_name() {
    let tmp = TempDir::new().unwrap();
    let bad = "---\nname: Bad_Name\ndescription: x\n---\nBody\n";
    let dir = write_skill(tmp.path(), "Bad_Name", bad);
    let err = parse_skill_dir(&dir, SkillSource::User);
    assert!(err.is_err());
}

// ── discover_skills ────────────────────────────────────────────────────────

#[test]
fn discover_skills_returns_all_valid_subdirs() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "alpha",
        "---\nname: alpha\ndescription: A\n---\nA body\n",
    );
    write_skill(
        tmp.path(),
        "beta",
        "---\nname: beta\ndescription: B\n---\nB body\n",
    );
    // A directory without a SKILL.md — should be skipped.
    std::fs::create_dir(tmp.path().join("gamma")).unwrap();

    let skills = discover_skills(tmp.path(), SkillSource::User);
    assert_eq!(skills.len(), 2);
    let names: Vec<_> = skills.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
}

#[test]
fn discover_skills_ignores_broken_skill() {
    let tmp = TempDir::new().unwrap();
    write_skill(
        tmp.path(),
        "good",
        "---\nname: good\ndescription: G\n---\nG body\n",
    );
    write_skill(tmp.path(), "broken", "No frontmatter at all");
    let skills = discover_skills(tmp.path(), SkillSource::User);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "good");
}

#[test]
fn discover_skills_returns_empty_for_missing_root() {
    let skills = discover_skills(std::path::Path::new("/no/such/dir"), SkillSource::User);
    assert!(skills.is_empty());
}

// ── embedded_builtin_skills ────────────────────────────────────────────────

#[test]
fn embedded_builtin_skills_returns_skills() {
    let skills = embedded_builtin_skills();
    // The repo bundles skills/, so the list should be non-empty.
    assert!(
        !skills.is_empty(),
        "expected at least one bundled built-in skill"
    );
    for skill in &skills {
        assert!(skill.is_builtin());
        assert!(!skill.name.is_empty());
        assert!(!skill.description.is_empty());
    }
}
