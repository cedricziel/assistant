//! Language hint for code views (design D7).
//!
//! Maps a file extension to the language id the client hands to its code
//! highlighter. The mapping is a hint only: an unknown extension yields
//! `None` and the client shows plain text.

use std::path::Path;

/// Extension (lowercase) to language id.
const LANGUAGES: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("dart", "dart"),
    ("ts", "typescript"),
    ("tsx", "tsx"),
    ("js", "javascript"),
    ("py", "python"),
    ("go", "go"),
    ("rb", "ruby"),
    ("java", "java"),
    ("kt", "kotlin"),
    ("swift", "swift"),
    ("c", "c"),
    ("h", "c"),
    ("cpp", "cpp"),
    ("cs", "csharp"),
    ("sh", "bash"),
    ("bash", "bash"),
    ("zsh", "bash"),
    ("json", "json"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("toml", "toml"),
    ("md", "markdown"),
    ("html", "html"),
    ("css", "css"),
    ("sql", "sql"),
    ("xml", "xml"),
];

/// The language id for the extension of `path`, matched case-insensitively.
pub(crate) fn language_for_path(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    LANGUAGES
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, language)| *language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_extensions() {
        let cases = [
            ("src/main.rs", "rust"),
            ("lib/app.dart", "dart"),
            ("a.ts", "typescript"),
            ("a.tsx", "tsx"),
            ("a.js", "javascript"),
            ("a.py", "python"),
            ("a.h", "c"),
            ("a.cpp", "cpp"),
            ("a.cs", "csharp"),
            ("run.zsh", "bash"),
            ("Cargo.toml", "toml"),
            ("ci.yml", "yaml"),
            ("README.md", "markdown"),
        ];
        for (path, language) in cases {
            assert_eq!(language_for_path(Path::new(path)), Some(language), "{path}");
        }
    }

    #[test]
    fn extension_match_ignores_case() {
        assert_eq!(language_for_path(Path::new("MAIN.RS")), Some("rust"));
    }

    #[test]
    fn unknown_or_missing_extension_has_no_language() {
        assert_eq!(language_for_path(Path::new("notes.xyz")), None);
        assert_eq!(language_for_path(Path::new("Makefile")), None);
        assert_eq!(language_for_path(Path::new(".gitignore")), None);
    }
}
