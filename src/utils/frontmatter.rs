use serde::de::DeserializeOwned;

/// YAML frontmatter delimiter
#[allow(dead_code)]
pub const FRONTMATTER_DELIMITER: &str = "---";

/// Result of parsing frontmatter from content
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParsedContent<T> {
    pub frontmatter: Option<T>,
    pub body: String,
    pub raw_content: String,
}

/// Splits content into raw frontmatter string and body, without parsing YAML.
/// Returns (frontmatter_str, body) if frontmatter exists, otherwise (None, original content).
#[allow(dead_code)]
pub fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let trimmed = content.trim_start();

    if !trimmed.starts_with(FRONTMATTER_DELIMITER) {
        return (None, content);
    }

    let mut parts = trimmed.splitn(3, FRONTMATTER_DELIMITER);
    parts.next(); // Skip empty string before first ---

    let frontmatter_str = match parts.next() {
        Some(s) => s.trim(),
        None => return (None, content),
    };

    let body = match parts.next() {
        Some(s) => s.trim_start(),
        None => return (None, content),
    };

    (Some(frontmatter_str), body)
}

/// Parses content with optional YAML frontmatter into a typed struct.
/// Returns ParsedContent with frontmatter (if valid YAML) and body.
#[allow(dead_code)]
pub fn parse_frontmatter<T: DeserializeOwned>(content: &str) -> ParsedContent<T> {
    let (frontmatter_str, body) = split_frontmatter(content);

    let frontmatter = frontmatter_str.and_then(|s| serde_yaml::from_str(s).ok());

    ParsedContent {
        frontmatter,
        body: body.to_string(),
        raw_content: content.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestFrontMatter {
        title: String,
        #[serde(default)]
        enabled: bool,
    }

    #[test]
    fn test_split_frontmatter_with_frontmatter() {
        let content = "---\ntitle: Test\n---\nBody content";
        let (fm, body) = split_frontmatter(content);
        assert_eq!(fm, Some("title: Test"));
        assert_eq!(body, "Body content");
    }

    #[test]
    fn test_split_frontmatter_without_frontmatter() {
        let content = "Just body content";
        let (fm, body) = split_frontmatter(content);
        assert_eq!(fm, None);
        assert_eq!(body, "Just body content");
    }

    #[test]
    fn test_parse_frontmatter_typed() {
        let content = "---\ntitle: Hello\nenabled: true\n---\nBody here";
        let parsed: ParsedContent<TestFrontMatter> = parse_frontmatter(content);
        assert_eq!(
            parsed.frontmatter,
            Some(TestFrontMatter {
                title: "Hello".to_string(),
                enabled: true
            })
        );
        assert_eq!(parsed.body, "Body here");
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml() {
        let content = "---\ninvalid: [unclosed\n---\nBody";
        let parsed: ParsedContent<TestFrontMatter> = parse_frontmatter(content);
        assert_eq!(parsed.frontmatter, None);
        assert_eq!(parsed.body, "Body");
    }
}
