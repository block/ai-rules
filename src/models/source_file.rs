use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::constants::GENERATED_FILE_PREFIX;

/// YAML frontmatter delimiter
const FRONTMATTER_DELIMITER: &str = "---";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    #[serde(default)]
    pub description: String,
    #[serde(rename = "alwaysApply")]
    pub always_apply: bool,
    #[serde(
        rename = "fileMatching",
        deserialize_with = "deserialize_comma_separated_optional",
        default
    )]
    pub file_matching_patterns: Option<Vec<String>>,
}

fn deserialize_comma_separated_optional<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.map(|s| {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }))
}

impl FrontMatter {
    fn with_defaults_from_path(file_path: &str) -> Self {
        let description = Path::new(file_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string())
            .unwrap_or_else(|| "Rule".to_string());

        Self {
            description,
            always_apply: true,
            file_matching_patterns: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub front_matter: FrontMatter,
    pub body: String,
    pub base_file_name: String,
}

impl SourceFile {
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let path = file_path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file '{}'", path.display()))?;
        let base_file_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Invalid filename for path: {}", path.display()))?
            .to_string();

        let file_path_str = path.display().to_string();
        let mut source_file = Self::parse(&content, &file_path_str)?;
        source_file.base_file_name = base_file_name;
        Ok(source_file)
    }

    pub fn get_body_file_name(&self) -> String {
        use std::path::Path;

        let p = Path::new(&self.base_file_name);
        let file = p.file_name().unwrap_or_default().to_string_lossy();
        let name = format!("{GENERATED_FILE_PREFIX}{file}.md");

        if let Some(parent) = p.parent() {
            parent.join(name).to_string_lossy().into_owned()
        } else {
            name
        }
    }

    fn parse(content: &str, file_path: &str) -> Result<Self> {
        let content = content.trim_start();

        if content.is_empty() {
            return Err(anyhow!("File '{}' is empty", file_path));
        }

        let has_frontmatter = content.starts_with(FRONTMATTER_DELIMITER);

        if !has_frontmatter {
            return Ok(SourceFile {
                front_matter: FrontMatter::with_defaults_from_path(file_path),
                body: content.to_string(),
                base_file_name: String::new(),
            });
        }

        if content.len() < FRONTMATTER_DELIMITER.len() {
            return Err(anyhow!(
                "File '{}' is too short to contain YAML frontmatter",
                file_path
            ));
        }

        let mut frontmatter_sections = content.splitn(3, FRONTMATTER_DELIMITER);

        frontmatter_sections.next();

        let frontmatter_str = frontmatter_sections.next().ok_or_else(|| {
            anyhow!(
                "Missing closing frontmatter delimiter '{}' in file '{}'",
                FRONTMATTER_DELIMITER,
                file_path
            )
        })?;

        let body = frontmatter_sections
            .next()
            .ok_or_else(|| {
                anyhow!(
                    "Missing body content after frontmatter in file '{}'",
                    file_path
                )
            })?
            .trim_start()
            .to_string();

        let front_matter: FrontMatter = serde_yaml::from_str(frontmatter_str)
            .with_context(|| format!("Failed to parse YAML frontmatter in file '{file_path}'. Ensure the YAML is valid and properly formatted"))?;

        Ok(SourceFile {
            front_matter,
            body,
            base_file_name: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let content = r#"---
description: Test rules
alwaysApply: true
fileMatching: "**/*.ts, **/*.tsx"
---

# Test Rules
This is a test body"#;

        let result = SourceFile::parse(content, "test.md").unwrap();

        assert_eq!(result.front_matter.description, "Test rules");
        assert!(result.front_matter.always_apply);
        assert_eq!(
            result.front_matter.file_matching_patterns,
            Some(vec!["**/*.ts".to_string(), "**/*.tsx".to_string()])
        );
        assert_eq!(result.body, "# Test Rules\nThis is a test body");
    }

    #[test]
    fn test_empty_file_matching() {
        let content = r#"---
description: Test rules
alwaysApply: true
fileMatching: 
---

# Test Rules
This is a test body"#;

        let result = SourceFile::parse(content, "test.md").unwrap();

        assert_eq!(result.front_matter.description, "Test rules");
        assert!(result.front_matter.always_apply);
        assert_eq!(result.front_matter.file_matching_patterns, None);
        assert_eq!(result.body, "# Test Rules\nThis is a test body");
    }

    #[test]
    fn test_parse_with_leading_whitespace() {
        let content = r#"

---
description: Test with whitespace
alwaysApply: false
fileMatching: "**/*.js"
---

# Body starts here"#;

        let result = SourceFile::parse(content, "test.md").unwrap();

        assert_eq!(result.front_matter.description, "Test with whitespace");
        assert!(!result.front_matter.always_apply);
        assert_eq!(
            result.front_matter.file_matching_patterns,
            Some(vec!["**/*.js".to_string()])
        );
        assert_eq!(result.body, "# Body starts here");
    }

    #[test]
    fn test_parse_with_trailing_newlines() {
        let content = r#"
---
description: Test with whitespace
alwaysApply: false
fileMatching: "**/*.js"
---

# Body starts here
"#;

        let result = SourceFile::parse(content, "test.md").unwrap();

        assert_eq!(result.front_matter.description, "Test with whitespace");
        assert!(!result.front_matter.always_apply);
        assert_eq!(
            result.front_matter.file_matching_patterns,
            Some(vec!["**/*.js".to_string()])
        );
        assert_eq!(result.body, "# Body starts here\n");
    }

    #[test]
    fn test_parse_no_file_matching_field() {
        let content = r#"---
description: Test rules
alwaysApply: true
---

# Test Rules
This is a test body"#;

        let result = SourceFile::parse(content, "test.md").unwrap();

        assert_eq!(result.front_matter.description, "Test rules");
        assert!(result.front_matter.always_apply);
        assert_eq!(result.front_matter.file_matching_patterns, None);
        assert_eq!(result.body, "# Test Rules\nThis is a test body");
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just markdown";

        let result = SourceFile::parse(content, "test.md").unwrap();
        assert_eq!(result.front_matter.description, "test");
        assert!(result.front_matter.always_apply);
        assert_eq!(result.front_matter.file_matching_patterns, None);
        assert_eq!(result.body, "# Just markdown");
    }
}
