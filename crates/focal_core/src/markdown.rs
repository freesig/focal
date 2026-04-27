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
