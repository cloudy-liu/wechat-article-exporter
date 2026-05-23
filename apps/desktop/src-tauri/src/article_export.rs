use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::archive_store::{
    ArchiveArticle, ArchiveArticleInput, ArchiveStore, ArchiveStoreError, CollectionTaskItemInput,
    CollectionTaskItemStatus, CollectionTaskType,
};

pub type ArticleExportResult<T> = Result<T, ArticleExportError>;

#[derive(Debug)]
pub enum ArticleExportError {
    ArchiveStore(ArchiveStoreError),
    ArticleNotFound(String),
    EmptyFormatList,
    Io(std::io::Error),
    MissingDownloadedHtml { article_id: String, path: PathBuf },
    Parse(serde_json::Error),
}

impl fmt::Display for ArticleExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveStore(error) => write!(formatter, "{error}"),
            Self::ArticleNotFound(article_id) => {
                write!(formatter, "article archive was not found: {article_id}")
            }
            Self::EmptyFormatList => write!(formatter, "at least one export format is required"),
            Self::Io(error) => write!(formatter, "article export filesystem error: {error}"),
            Self::MissingDownloadedHtml { article_id, path } => write!(
                formatter,
                "downloaded article HTML is missing for {article_id}: {}",
                path.display()
            ),
            Self::Parse(error) => write!(formatter, "article export payload failed: {error}"),
        }
    }
}

impl std::error::Error for ArticleExportError {}

impl From<ArchiveStoreError> for ArticleExportError {
    fn from(error: ArchiveStoreError) -> Self {
        Self::ArchiveStore(error)
    }
}

impl From<std::io::Error> for ArticleExportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ArticleExportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parse(error)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArticleExportFormat {
    Markdown,
    Html,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleExportRequest {
    pub article_id: String,
    pub formats: Vec<ArticleExportFormat>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleExportOutcome {
    pub article_id: String,
    pub source_html_file: PathBuf,
    pub markdown_file: Option<PathBuf>,
    pub html_file: Option<PathBuf>,
    pub task_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleArchivePreviewRequest {
    pub article_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleArchivePreview {
    pub article_id: String,
    pub title: String,
    pub source_url: String,
    pub source_html_file: PathBuf,
    pub html: String,
}

pub struct ArticleExportService;

impl ArticleExportService {
    pub fn new() -> Self {
        Self
    }

    pub fn export_article(
        &self,
        archive_store: &ArchiveStore,
        request: ArticleExportRequest,
    ) -> ArticleExportResult<ArticleExportOutcome> {
        if request.formats.is_empty() {
            return Err(ArticleExportError::EmptyFormatList);
        }

        let article = archive_store
            .get_article(&request.article_id)?
            .ok_or_else(|| ArticleExportError::ArticleNotFound(request.article_id.clone()))?;
        let task = archive_store.create_collection_task(
            CollectionTaskType::Export,
            Some(&article.target_account_id),
            vec![CollectionTaskItemInput {
                item_id: article.article_id.clone(),
                item_type: "article-export".to_string(),
                payload_json: serde_json::to_string(&request)?,
            }],
        )?;
        archive_store.update_collection_task_item_status(
            &task.task_id,
            &article.article_id,
            CollectionTaskItemStatus::Running,
            None,
        )?;

        match self.export_article_with_task(
            archive_store,
            &article,
            &request.formats,
            &task.task_id,
        ) {
            Ok(outcome) => {
                archive_store.update_collection_task_item_status(
                    &task.task_id,
                    &article.article_id,
                    CollectionTaskItemStatus::Succeeded,
                    None,
                )?;
                Ok(outcome)
            }
            Err(error) => {
                archive_store.update_collection_task_item_status(
                    &task.task_id,
                    &article.article_id,
                    CollectionTaskItemStatus::Failed,
                    Some(error.to_string()),
                )?;
                Err(error)
            }
        }
    }

    fn export_article_with_task(
        &self,
        archive_store: &ArchiveStore,
        article: &ArchiveArticle,
        formats: &[ArticleExportFormat],
        task_id: &str,
    ) -> ArticleExportResult<ArticleExportOutcome> {
        let source_html_file =
            article
                .html_file
                .clone()
                .ok_or_else(|| ArticleExportError::MissingDownloadedHtml {
                    article_id: article.article_id.clone(),
                    path: PathBuf::from("(empty html_file)"),
                })?;
        let source_html_path = archive_store.archive_dir().join(&source_html_file);
        if !source_html_path.is_file() {
            return Err(ArticleExportError::MissingDownloadedHtml {
                article_id: article.article_id.clone(),
                path: source_html_file,
            });
        }

        let raw_html = fs::read_to_string(archive_store.archive_dir().join(&source_html_file))?;
        let export_dir = Path::new("exports").to_path_buf();
        let output_stem = export_file_stem(&article.title, &article.article_id);
        let asset_map = local_asset_reference_map(archive_store, article, &export_dir)?;
        let cleaned_article_html = clean_article_html(&raw_html, &asset_map);
        let mut markdown_file = None;
        let mut html_file = None;

        if formats.contains(&ArticleExportFormat::Markdown) {
            let path = export_dir.join(format!("{output_stem}.md"));
            write_archive_file(
                archive_store,
                &path,
                render_markdown(article, &cleaned_article_html),
            )?;
            markdown_file = Some(path);
        }

        if formats.contains(&ArticleExportFormat::Html) {
            let path = export_dir.join(format!("{output_stem}.html"));
            write_archive_file(
                archive_store,
                &path,
                render_html_document(&cleaned_article_html),
            )?;
            html_file = Some(path);
        }

        archive_store.upsert_article(&ArchiveArticleInput {
            article_id: article.article_id.clone(),
            target_account_id: article.target_account_id.clone(),
            title: article.title.clone(),
            source_url: article.source_url.clone(),
            html_file: article.html_file.clone(),
            markdown_file: markdown_file
                .clone()
                .or_else(|| article.markdown_file.clone()),
        })?;

        Ok(ArticleExportOutcome {
            article_id: article.article_id.clone(),
            source_html_file,
            markdown_file,
            html_file,
            task_id: task_id.to_string(),
        })
    }
}

pub struct ArticleArchivePreviewService;

impl ArticleArchivePreviewService {
    pub fn new() -> Self {
        Self
    }

    pub fn preview_article(
        &self,
        archive_store: &ArchiveStore,
        request: ArticleArchivePreviewRequest,
    ) -> ArticleExportResult<ArticleArchivePreview> {
        let article = archive_store
            .get_article(&request.article_id)?
            .ok_or_else(|| ArticleExportError::ArticleNotFound(request.article_id.clone()))?;
        let source_html_file =
            article
                .html_file
                .clone()
                .ok_or_else(|| ArticleExportError::MissingDownloadedHtml {
                    article_id: article.article_id.clone(),
                    path: PathBuf::from("(empty html_file)"),
                })?;
        let source_html_path = archive_store.archive_dir().join(&source_html_file);
        if !source_html_path.is_file() {
            return Err(ArticleExportError::MissingDownloadedHtml {
                article_id: article.article_id.clone(),
                path: source_html_file,
            });
        }

        let raw_html = fs::read_to_string(archive_store.archive_dir().join(&source_html_file))?;
        let preview_dir = Path::new("exports").to_path_buf();
        let asset_map = local_asset_reference_map(archive_store, &article, &preview_dir)?;
        let cleaned_article_html = clean_article_html(&raw_html, &asset_map);

        Ok(ArticleArchivePreview {
            article_id: article.article_id,
            title: article.title,
            source_url: article.source_url,
            source_html_file,
            html: render_html_document(&cleaned_article_html),
        })
    }
}

fn write_archive_file(
    archive_store: &ArchiveStore,
    relative_path: &Path,
    content: String,
) -> ArticleExportResult<()> {
    let absolute_path = archive_store.archive_dir().join(relative_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(absolute_path, content)?;

    Ok(())
}

fn render_html_document(article_html: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="referrer" content="no-referrer">
  <style>
    body {{
      margin: 0 auto;
      max-width: 720px;
      padding: 32px 20px;
      font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      line-height: 1.7;
      color: #1f2a24;
    }}
    img {{
      max-width: 100%;
      height: auto;
    }}
  </style>
</head>
<body>
{}
</body>
</html>
"#,
        article_html.trim()
    )
}

fn render_markdown(article: &ArchiveArticle, article_html: &str) -> String {
    let content_html = extract_element_by_id(article_html, "js_content")
        .unwrap_or_else(|| article_html.to_string());
    let mut markdown = String::new();
    markdown.push_str("# ");
    markdown.push_str(&article.title);
    markdown.push_str("\n\n");
    markdown.push_str(&html_fragment_to_markdown(&content_html));
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }

    markdown
}

fn html_fragment_to_markdown(html: &str) -> String {
    let mut fragment = remove_element_by_tag(html, "script");
    fragment = replace_img_tags_with_markdown(&fragment);
    for tag in ["p", "div", "section", "blockquote", "li", "br"] {
        fragment = replace_tag_boundaries(&fragment, tag);
    }
    fragment = strip_tags(&fragment);
    fragment = decode_html_entities(&fragment);
    normalize_markdown_whitespace(&fragment)
}

fn clean_article_html(raw_html: &str, asset_map: &HashMap<String, String>) -> String {
    let mut article_html =
        extract_element_by_id(raw_html, "js_article").unwrap_or_else(|| raw_html.to_string());
    article_html = remove_element_by_tag(&article_html, "script");
    for id in [
        "js_top_ad_area",
        "js_tags_preview_toast",
        "content_bottom_area",
        "js_pc_qr_code",
        "wx_stream_article_slide_tip",
    ] {
        article_html = remove_element_by_id(&article_html, id);
    }
    article_html = remove_attribute_from_element_by_id(&article_html, "js_content", "style");
    rewrite_img_tags(&article_html, asset_map)
}

fn local_asset_reference_map(
    archive_store: &ArchiveStore,
    article: &ArchiveArticle,
    export_dir: &Path,
) -> ArticleExportResult<HashMap<String, String>> {
    let asset_dir = Path::new("assets")
        .join(safe_file_segment(&article.target_account_id))
        .join(safe_file_segment(&article.article_id));
    let absolute_asset_dir = archive_store.archive_dir().join(&asset_dir);
    let mut map = HashMap::new();
    let entries = match fs::read_dir(&absolute_asset_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(map),
        Err(error) => return Err(error.into()),
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let relative_asset_file = asset_dir.join(&file_name);
        let html_reference =
            path_to_html_reference(&relative_path(export_dir, &relative_asset_file));
        map.insert(file_name.clone(), html_reference.clone());
        if let Some(stripped) = strip_numbered_asset_prefix(&file_name) {
            map.insert(stripped.to_string(), html_reference);
        }
    }

    Ok(map)
}

fn rewrite_img_tags(html: &str, asset_map: &HashMap<String, String>) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(relative_start) = html[cursor..].find("<img") {
        let start = cursor + relative_start;
        let Some(relative_end) = html[start..].find('>') else {
            break;
        };
        let end = start + relative_end + 1;
        output.push_str(&html[cursor..start]);
        output.push_str(&rewrite_img_tag(&html[start..end], asset_map));
        cursor = end;
    }

    output.push_str(&html[cursor..]);
    output
}

fn rewrite_img_tag(tag: &str, asset_map: &HashMap<String, String>) -> String {
    let source = attribute_value(tag, "data-src").or_else(|| attribute_value(tag, "src"));
    let local_source = source
        .as_deref()
        .and_then(|value| local_asset_reference(value, asset_map))
        .or(source);
    let Some(local_source) = local_source else {
        return tag.to_string();
    };
    let tag = remove_attribute(tag, "data-src");
    let tag = remove_attribute(&tag, "src");
    insert_attribute(&tag, "src", &local_source)
}

fn replace_img_tags_with_markdown(html: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(relative_start) = html[cursor..].find("<img") {
        let start = cursor + relative_start;
        let Some(relative_end) = html[start..].find('>') else {
            break;
        };
        let end = start + relative_end + 1;
        output.push_str(&html[cursor..start]);
        let tag = &html[start..end];
        if let Some(source) =
            attribute_value(tag, "src").or_else(|| attribute_value(tag, "data-src"))
        {
            let alt = attribute_value(tag, "alt").unwrap_or_else(|| image_alt_from_source(&source));
            output.push_str("\n\n![");
            output.push_str(&decode_html_entities(&alt));
            output.push_str("](");
            output.push_str(&source);
            output.push_str(")\n\n");
        }
        cursor = end;
    }

    output.push_str(&html[cursor..]);
    output
}

fn replace_tag_boundaries(html: &str, tag: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;

    while let Some(relative_start) = html[cursor..].find('<') {
        let start = cursor + relative_start;
        output.push_str(&html[cursor..start]);
        let Some(relative_end) = html[start..].find('>') else {
            output.push_str(&html[start..]);
            return output;
        };
        let end = start + relative_end + 1;
        if starts_with_tag_at(html, start, tag, false) || starts_with_tag_at(html, start, tag, true)
        {
            output.push('\n');
        } else {
            output.push_str(&html[start..end]);
        }
        cursor = end;
    }

    output.push_str(&html[cursor..]);
    output
}

fn strip_tags(html: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;

    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }

    output
}

fn normalize_markdown_whitespace(value: &str) -> String {
    let mut lines = Vec::new();
    let mut previous_blank = true;

    for line in value.lines() {
        let line = collapse_inline_whitespace(line).trim().to_string();
        if line.is_empty() {
            if !previous_blank {
                lines.push(String::new());
            }
            previous_blank = true;
        } else {
            lines.push(line);
            previous_blank = false;
        }
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

fn collapse_inline_whitespace(value: &str) -> String {
    let mut output = String::new();
    let mut previous_space = false;

    for character in value.chars() {
        if character.is_whitespace() {
            if !previous_space {
                output.push(' ');
            }
            previous_space = true;
        } else {
            output.push(character);
            previous_space = false;
        }
    }

    output
}

fn remove_element_by_tag(html: &str, tag: &str) -> String {
    let mut output = html.to_string();
    while let Some(start) = find_tag_start(&output, tag) {
        let end = matching_element_end(&output, start, tag)
            .or_else(|| {
                output[start..]
                    .find('>')
                    .map(|position| start + position + 1)
            })
            .unwrap_or(output.len());
        output.replace_range(start..end, "");
    }

    output
}

fn remove_element_by_id(html: &str, id: &str) -> String {
    let mut output = html.to_string();
    while let Some((start, end, _tag)) = element_range_by_id(&output, id) {
        output.replace_range(start..end, "");
    }

    output
}

fn remove_attribute_from_element_by_id(html: &str, id: &str, attribute: &str) -> String {
    let Some((start, _end, _tag)) = element_range_by_id(html, id) else {
        return html.to_string();
    };
    let Some(open_end) = html[start..].find('>').map(|position| start + position + 1) else {
        return html.to_string();
    };
    let mut output = String::new();
    output.push_str(&html[..start]);
    output.push_str(&remove_attribute(&html[start..open_end], attribute));
    output.push_str(&html[open_end..]);
    output
}

fn extract_element_by_id(html: &str, id: &str) -> Option<String> {
    let (start, end, _tag) = element_range_by_id(html, id)?;
    Some(html[start..end].to_string())
}

fn element_range_by_id(html: &str, id: &str) -> Option<(usize, usize, String)> {
    let id_position = find_id_position(html, id)?;
    let start = html[..id_position].rfind('<')?;
    let tag = tag_name_at(html, start)?;
    let end = matching_element_end(html, start, &tag)
        .or_else(|| html[start..].find('>').map(|position| start + position + 1))?;

    Some((start, end, tag))
}

fn find_id_position(html: &str, id: &str) -> Option<usize> {
    for pattern in [format!("id=\"{id}\""), format!("id='{id}'")] {
        if let Some(position) = html.find(&pattern) {
            return Some(position);
        }
    }

    None
}

fn find_tag_start(html: &str, tag: &str) -> Option<usize> {
    let mut cursor = 0;
    while let Some(relative_start) = html[cursor..].find('<') {
        let start = cursor + relative_start;
        if starts_with_tag_at(html, start, tag, false) {
            return Some(start);
        }
        cursor = start + 1;
    }

    None
}

fn matching_element_end(html: &str, start: usize, tag: &str) -> Option<usize> {
    let open_end = html[start..]
        .find('>')
        .map(|position| start + position + 1)?;
    if html[start..open_end].trim_end().ends_with("/>") {
        return Some(open_end);
    }

    let mut depth = 1usize;
    let mut cursor = open_end;
    while let Some(relative_next) = html[cursor..].find('<') {
        let next = cursor + relative_next;
        if starts_with_tag_at(html, next, tag, true) {
            depth -= 1;
            let close_end = html[next..].find('>').map(|position| next + position + 1)?;
            if depth == 0 {
                return Some(close_end);
            }
            cursor = close_end;
        } else if starts_with_tag_at(html, next, tag, false) {
            let nested_open_end = html[next..].find('>').map(|position| next + position + 1)?;
            if !html[next..nested_open_end].trim_end().ends_with("/>") {
                depth += 1;
            }
            cursor = nested_open_end;
        } else {
            cursor = next + 1;
        }
    }

    None
}

fn starts_with_tag_at(html: &str, position: usize, tag: &str, closing: bool) -> bool {
    let prefix = if closing { "</" } else { "<" };
    let Some(candidate) = html[position..].strip_prefix(prefix) else {
        return false;
    };
    if !candidate
        .get(..tag.len())
        .is_some_and(|value| value.eq_ignore_ascii_case(tag))
    {
        return false;
    }
    candidate
        .chars()
        .nth(tag.len())
        .is_some_and(|character| character.is_whitespace() || matches!(character, '>' | '/'))
}

fn tag_name_at(html: &str, start: usize) -> Option<String> {
    let mut name = String::new();
    for character in html[start + 1..].chars() {
        if character.is_whitespace() || matches!(character, '>' | '/') {
            break;
        }
        name.push(character.to_ascii_lowercase());
    }

    (!name.is_empty()).then_some(name)
}

fn attribute_value(tag: &str, attribute: &str) -> Option<String> {
    find_attribute_range(tag, attribute).map(|range| range.value)
}

struct AttributeRange {
    start: usize,
    end: usize,
    value: String,
}

fn find_attribute_range(tag: &str, attribute: &str) -> Option<AttributeRange> {
    let bytes = tag.as_bytes();
    let mut cursor = 0;

    while cursor < bytes.len() {
        let relative = tag[cursor..].find(attribute)?;
        let name_start = cursor + relative;
        let name_end = name_start + attribute.len();
        let before = tag[..name_start].chars().next_back();
        let after = tag[name_end..].chars().next();
        let valid_before =
            before.is_some_and(|character| character.is_whitespace() || character == '<');
        let valid_after =
            after.is_some_and(|character| character.is_whitespace() || character == '=');
        if !valid_before || !valid_after {
            cursor = name_end;
            continue;
        }

        let mut value_start = name_end;
        while tag[value_start..].starts_with(char::is_whitespace) {
            value_start += tag[value_start..].chars().next()?.len_utf8();
        }
        if !tag[value_start..].starts_with('=') {
            cursor = name_end;
            continue;
        }
        value_start += 1;
        while tag[value_start..].starts_with(char::is_whitespace) {
            value_start += tag[value_start..].chars().next()?.len_utf8();
        }

        let quote = tag[value_start..].chars().next()?;
        let (value, value_end) = if quote == '"' || quote == '\'' {
            let content_start = value_start + quote.len_utf8();
            let relative_end = tag[content_start..].find(quote)?;
            let content_end = content_start + relative_end;
            (
                tag[content_start..content_end].to_string(),
                content_end + quote.len_utf8(),
            )
        } else {
            let content_end = tag[value_start..]
                .find(|character: char| character.is_whitespace() || matches!(character, '>' | '/'))
                .map(|relative_end| value_start + relative_end)
                .unwrap_or(tag.len());
            (tag[value_start..content_end].to_string(), content_end)
        };
        let remove_start = tag[..name_start]
            .chars()
            .next_back()
            .filter(|character| character.is_whitespace())
            .map(|character| name_start - character.len_utf8())
            .unwrap_or(name_start);

        return Some(AttributeRange {
            start: remove_start,
            end: value_end,
            value,
        });
    }

    None
}

fn remove_attribute(tag: &str, attribute: &str) -> String {
    let mut output = tag.to_string();
    while let Some(range) = find_attribute_range(&output, attribute) {
        output.replace_range(range.start..range.end, "");
    }

    output
}

fn insert_attribute(tag: &str, attribute: &str, value: &str) -> String {
    let escaped = escape_html_attribute(value);
    let insertion = format!(" {attribute}=\"{escaped}\"");
    if let Some(position) = tag.rfind("/>") {
        format!("{}{}{}", &tag[..position], insertion, &tag[position..])
    } else if let Some(position) = tag.rfind('>') {
        format!("{}{}{}", &tag[..position], insertion, &tag[position..])
    } else {
        tag.to_string()
    }
}

fn local_asset_reference(value: &str, asset_map: &HashMap<String, String>) -> Option<String> {
    for key in asset_lookup_keys(value) {
        if let Some(reference) = asset_map.get(&key) {
            return Some(reference.clone());
        }
    }

    None
}

fn asset_lookup_keys(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let normalized = if trimmed.starts_with("//") {
        format!("https:{trimmed}")
    } else {
        trimmed.to_string()
    };
    let mut keys = Vec::new();
    if let Ok(url) = Url::parse(&normalized) {
        if let Some(segment) = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|segment| !segment.is_empty())
        {
            keys.push(safe_file_segment(segment));
        }
    }
    if let Some(file_name) = Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
    {
        keys.push(safe_file_segment(file_name));
    }

    keys
}

fn strip_numbered_asset_prefix(file_name: &str) -> Option<&str> {
    let (prefix, rest) = file_name.split_once('-')?;
    (prefix.len() == 3 && prefix.chars().all(|character| character.is_ascii_digit()))
        .then_some(rest)
}

fn image_alt_from_source(source: &str) -> String {
    let file_name = source
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("image");
    let file_name = strip_numbered_asset_prefix(file_name).unwrap_or(file_name);
    let stem = file_name.split('.').next().unwrap_or(file_name);
    let mut output = String::new();
    let mut capitalize_next = true;

    for character in stem.chars() {
        if matches!(character, '-' | '_' | '.') {
            output.push(' ');
            capitalize_next = true;
        } else if capitalize_next {
            output.extend(character.to_uppercase());
            capitalize_next = false;
        } else {
            output.push(character);
        }
    }

    let output = output.trim();
    if output.is_empty() {
        "Image".to_string()
    } else {
        output.to_string()
    }
}

fn export_file_stem(title: &str, article_id: &str) -> String {
    format!(
        "{}-{}",
        safe_file_segment_with_default(title, "article"),
        safe_file_segment_with_default(article_id, "item")
    )
}

fn safe_file_segment(value: &str) -> String {
    safe_file_segment_with_default(value, "item")
}

fn safe_file_segment_with_default(value: &str, default: &str) -> String {
    let mut output = String::new();
    let mut previous_separator = false;

    for character in value.chars() {
        let safe_character = if character.is_control()
            || character.is_whitespace()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) {
            '_'
        } else {
            character
        };

        if safe_character == '_' {
            if !previous_separator {
                output.push('_');
                previous_separator = true;
            }
        } else {
            output.push(safe_character);
            previous_separator = false;
        }
    }

    let output = output.trim_matches(['.', '_', ' ']);
    if output.is_empty() {
        default.to_string()
    } else {
        output.chars().take(96).collect()
    }
}

fn relative_path(from_dir: &Path, target: &Path) -> PathBuf {
    let from_components = normal_components(from_dir);
    let target_components = normal_components(target);
    let common_len = from_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in common_len..from_components.len() {
        relative.push("..");
    }
    for component in &target_components[common_len..] {
        relative.push(component);
    }

    relative
}

fn normal_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

fn path_to_html_reference(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            std::path::Component::ParentDir => Some("..".to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
