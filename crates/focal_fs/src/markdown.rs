use std::collections::BTreeMap;
use std::path::Path;

use crate::model::{NodeContent, NodeKind};

#[derive(Debug, Clone)]
pub(crate) struct ParsedMarkdown {
    pub id: String,
    pub kind: NodeKind,
    pub title: String,
    pub content: NodeContent,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedContextMarkdown {
    pub id: String,
    pub title: String,
    pub markdown: String,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

pub(crate) fn parse_node_markdown(path: &Path, source: &str) -> Result<ParsedMarkdown, String> {
    let (metadata, body) = split_front_matter(source)?;
    let id = required_field(&metadata, "id")?.to_string();
    let kind = match required_field(&metadata, "kind")? {
        "statement" => NodeKind::Statement,
        "qa" => NodeKind::QuestionAnswer,
        other => return Err(format!("unsupported kind `{other}`")),
    };
    let title = required_field(&metadata, "title")?.to_string();
    let created_at_unix = parse_unix(required_field(&metadata, "created_at_unix")?)?;
    let updated_at_unix = parse_unix(required_field(&metadata, "updated_at_unix")?)?;

    let content = match kind {
        NodeKind::Statement => NodeContent::Statement {
            body: body.to_string(),
        },
        NodeKind::QuestionAnswer => {
            let (question, answer) = parse_question_answer(path, body)?;
            NodeContent::QuestionAnswer { question, answer }
        }
    };

    Ok(ParsedMarkdown {
        id,
        kind,
        title,
        content,
        created_at_unix,
        updated_at_unix,
    })
}

pub(crate) fn parse_context_markdown(source: &str) -> Result<ParsedContextMarkdown, String> {
    let (metadata, body) = split_front_matter(source)?;
    let id = required_field(&metadata, "id")?.to_string();
    let title = required_field(&metadata, "title")?.to_string();
    let created_at_unix = parse_unix(required_field(&metadata, "created_at_unix")?)?;
    let updated_at_unix = parse_unix(required_field(&metadata, "updated_at_unix")?)?;

    Ok(ParsedContextMarkdown {
        id,
        title,
        markdown: body.to_string(),
        created_at_unix,
        updated_at_unix,
    })
}

pub(crate) fn render_node_markdown(
    id: &str,
    kind: &NodeKind,
    title: &str,
    created_at_unix: u64,
    updated_at_unix: u64,
    content: &NodeContent,
) -> String {
    let kind_value = match kind {
        NodeKind::Statement => "statement",
        NodeKind::QuestionAnswer => "qa",
    };
    let body = match content {
        NodeContent::Statement { body } => body.clone(),
        NodeContent::QuestionAnswer { question, answer } => {
            format!(
                "## Question\n\n{}\n\n## Answer\n\n{}",
                trim_trailing_line_endings(question),
                trim_trailing_line_endings(answer)
            )
        }
    };

    format!(
        "---\nid: {id}\nkind: {kind_value}\ntitle: {title}\ncreated_at_unix: {created_at_unix}\nupdated_at_unix: {updated_at_unix}\n---\n\n{body}"
    )
}

pub(crate) fn render_context_markdown(
    id: &str,
    title: &str,
    created_at_unix: u64,
    updated_at_unix: u64,
    markdown: &str,
) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\ncreated_at_unix: {created_at_unix}\nupdated_at_unix: {updated_at_unix}\n---\n\n{markdown}"
    )
}

fn split_front_matter(source: &str) -> Result<(BTreeMap<String, String>, &str), String> {
    let mut lines = source.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return Err("missing metadata block".to_string());
    };
    if strip_line_ending(first) != "---" {
        return Err("missing opening metadata delimiter".to_string());
    }

    let mut consumed = first.len();
    let mut metadata = BTreeMap::new();
    let mut found_end = false;

    for line in lines {
        consumed += line.len();
        let stripped = strip_line_ending(line);
        if stripped == "---" {
            found_end = true;
            break;
        }
        let Some((key, value)) = stripped.split_once(':') else {
            return Err(format!("invalid metadata line `{stripped}`"));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err("empty metadata key".to_string());
        }
        if metadata
            .insert(key.to_string(), value.trim_start().to_string())
            .is_some()
        {
            return Err(format!("duplicate metadata field `{key}`"));
        }
    }

    if !found_end {
        return Err("missing closing metadata delimiter".to_string());
    }

    let mut body = &source[consumed..];
    if let Some(stripped) = body.strip_prefix("\r\n") {
        body = stripped;
    } else if let Some(stripped) = body.strip_prefix('\n') {
        body = stripped;
    }

    Ok((metadata, body))
}

fn required_field<'a>(
    metadata: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, String> {
    metadata
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing metadata field `{key}`"))
}

fn parse_unix(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid unix timestamp `{value}`"))
}

fn parse_question_answer(path: &Path, body: &str) -> Result<(String, String), String> {
    let question = find_heading(body, "## Question", 0)
        .ok_or_else(|| format!("{} is missing `## Question`", path.display()))?;
    let answer = find_heading(body, "## Answer", question.content_start)
        .ok_or_else(|| format!("{} is missing `## Answer`", path.display()))?;

    if answer.heading_start < question.content_start {
        return Err("`## Answer` appears before `## Question`".to_string());
    }

    let question_text = normalize_section(&body[question.content_start..answer.heading_start]);
    let answer_text = normalize_section(&body[answer.content_start..]);
    if question_text.trim().is_empty() {
        return Err("question must not be empty".to_string());
    }

    Ok((question_text, answer_text))
}

#[derive(Debug, Clone, Copy)]
struct Heading {
    heading_start: usize,
    content_start: usize,
}

fn find_heading(source: &str, heading: &str, start: usize) -> Option<Heading> {
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let line_start = offset;
        let line_end = offset + line.len();
        if line_start >= start && strip_line_ending(line).trim() == heading {
            return Some(Heading {
                heading_start: line_start,
                content_start: line_end,
            });
        }
        offset = line_end;
    }

    if offset >= start && source[offset..].trim() == heading {
        Some(Heading {
            heading_start: offset,
            content_start: source.len(),
        })
    } else {
        None
    }
}

fn normalize_section(section: &str) -> String {
    let section = match section.strip_prefix("\r\n") {
        Some(stripped) => stripped,
        None => match section.strip_prefix('\n') {
            Some(stripped) => stripped,
            None => section,
        },
    };
    trim_trailing_line_endings(section).to_string()
}

fn strip_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn trim_trailing_line_endings(value: &str) -> &str {
    value.trim_end_matches(['\r', '\n'])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn metadata(id: &str, kind: &str, title: &str) -> String {
        format!(
            "---\nid: {id}\nkind: {kind}\ntitle: {title}\ncreated_at_unix: 1\nupdated_at_unix: 2\n---\n\n"
        )
    }

    #[test]
    fn spec_08_statement_markdown_round_trips_without_heading_management() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let content = NodeContent::Statement {
            body: "# Human heading\n\nBody text\n".to_string(),
        };

        let rendered = render_node_markdown(id, &NodeKind::Statement, "A Title", 10, 11, &content);

        assert!(rendered.starts_with(
            "---\nid: 550e8400-e29b-41d4-a716-446655440000\nkind: statement\ntitle: A Title\ncreated_at_unix: 10\nupdated_at_unix: 11\n---\n\n"
        ));
        assert!(rendered.contains("# Human heading"));

        let parsed = parse_node_markdown(Path::new("node.md"), &rendered).unwrap();
        assert_eq!(parsed.id, id);
        assert_eq!(parsed.kind, NodeKind::Statement);
        assert_eq!(parsed.title, "A Title");
        assert_eq!(parsed.content, content);
    }

    #[test]
    fn spec_08_question_answer_markdown_requires_managed_sections() {
        let id = "7d9f2e5c-0f22-4c18-a0be-9f23e772a0bc";
        let rendered = render_node_markdown(
            id,
            &NodeKind::QuestionAnswer,
            "Why",
            1,
            1,
            &NodeContent::QuestionAnswer {
                question: "Why use symlinks?".to_string(),
                answer: String::new(),
            },
        );

        let parsed = parse_node_markdown(Path::new("qa.md"), &rendered).unwrap();
        assert_eq!(
            parsed.content,
            NodeContent::QuestionAnswer {
                question: "Why use symlinks?".to_string(),
                answer: String::new(),
            }
        );

        let missing_answer = format!(
            "{}## Question\n\nWhy?",
            metadata(id, "qa", "Missing answer")
        );
        assert!(
            parse_node_markdown(Path::new("qa.md"), &missing_answer)
                .unwrap_err()
                .contains("missing `## Answer`")
        );

        let empty_question = format!(
            "{}## Question\n\n\n## Answer\n\nLater",
            metadata(id, "qa", "Empty")
        );
        assert_eq!(
            parse_node_markdown(Path::new("qa.md"), &empty_question).unwrap_err(),
            "question must not be empty"
        );
    }

    #[test]
    fn spec_08_markdown_parser_rejects_invalid_metadata_blocks() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let duplicate = format!(
            "---\nid: {id}\nkind: statement\nkind: statement\ntitle: Duplicate\ncreated_at_unix: 1\nupdated_at_unix: 1\n---\n\nBody"
        );
        assert_eq!(
            parse_node_markdown(Path::new("node.md"), &duplicate).unwrap_err(),
            "duplicate metadata field `kind`"
        );

        let invalid_timestamp = format!(
            "---\nid: {id}\nkind: statement\ntitle: Bad time\ncreated_at_unix: now\nupdated_at_unix: 1\n---\n\nBody"
        );
        assert_eq!(
            parse_node_markdown(Path::new("node.md"), &invalid_timestamp).unwrap_err(),
            "invalid unix timestamp `now`"
        );
    }

    #[test]
    fn spec_08_context_markdown_round_trips_without_heading_management() {
        let id = "7a736f79-bf3f-4d1e-8bd8-71fd9b94a2d4";
        let markdown = "# Human heading\n\nRaw notes\n";

        let rendered = render_context_markdown(id, "Raw planning notes", 10, 11, markdown);

        assert!(rendered.starts_with(
            "---\nid: 7a736f79-bf3f-4d1e-8bd8-71fd9b94a2d4\ntitle: Raw planning notes\ncreated_at_unix: 10\nupdated_at_unix: 11\n---\n\n"
        ));
        assert!(rendered.contains("# Human heading"));

        let parsed = parse_context_markdown(&rendered).unwrap();
        assert_eq!(parsed.id, id);
        assert_eq!(parsed.title, "Raw planning notes");
        assert_eq!(parsed.markdown, markdown);
        assert_eq!(parsed.created_at_unix, 10);
        assert_eq!(parsed.updated_at_unix, 11);
    }

    #[test]
    fn spec_08_context_markdown_allows_empty_body_and_requires_metadata() {
        let id = "7a736f79-bf3f-4d1e-8bd8-71fd9b94a2d4";
        let rendered = render_context_markdown(id, "Empty", 1, 1, "");

        let parsed = parse_context_markdown(&rendered).unwrap();
        assert_eq!(parsed.markdown, "");
        assert_eq!(
            parse_context_markdown("---\nid: bad\ncreated_at_unix: 1\nupdated_at_unix: 1\n---\n")
                .unwrap_err(),
            "missing metadata field `title`"
        );
    }
}
