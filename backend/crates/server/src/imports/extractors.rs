use std::{
    fs::{self, File},
    io::{Read, Seek},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{bail, Context};
use quick_xml::{events::Event, Reader};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use wait_timeout::ChildExt;
use zip::ZipArchive;

const MAX_TEXT_CHARS: usize = 128 * 1024;
const MAX_PLAIN_TEXT_BYTES: u64 = 256 * 1024;
const MAX_INNER_ARCHIVE_ENTRIES: usize = 2_000;
const MAX_INNER_XML_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PDF_BYTES: u64 = 32 * 1024 * 1024;
const FFPROBE_TIMEOUT: Duration = Duration::from_secs(8);
const PREVIEW_TIMEOUT: Duration = Duration::from_secs(20);

pub(super) struct ArtifactExtraction {
    pub extractor: String,
    pub text: Option<String>,
    pub metadata: Value,
}

pub(super) struct GeneratedPreview {
    pub output_path: std::path::PathBuf,
    pub extractor: &'static str,
    pub metadata: Value,
}

impl ArtifactExtraction {
    fn indexed(metadata: Value) -> Self {
        Self {
            extractor: "file_index".to_owned(),
            text: None,
            metadata,
        }
    }

    fn failed(extractor: &str, message: &str) -> Self {
        Self {
            extractor: format!("{extractor}_failed"),
            text: None,
            metadata: json!({ "status": "error", "message": message }),
        }
    }
}

pub(super) fn extract_artifact(
    path: &Path,
    relative_path: &Path,
    kind: &str,
    size_bytes: u64,
    ffprobe_bin: &str,
) -> ArtifactExtraction {
    let extension = relative_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "docx" => extract_docx(path).unwrap_or_else(|error| {
            tracing::warn!(file = %relative_path.display(), error = %error, "DOCX extraction failed");
            ArtifactExtraction::failed("docx_text", "DOCX 正文读取失败，已保留原文件")
        }),
        "pptx" => extract_pptx(path).unwrap_or_else(|error| {
            tracing::warn!(file = %relative_path.display(), error = %error, "PPTX extraction failed");
            ArtifactExtraction::failed("pptx_text", "PPTX 内容读取失败，已保留原文件")
        }),
        "pdf" if size_bytes <= MAX_PDF_BYTES => extract_pdf(path).unwrap_or_else(|error| {
            tracing::warn!(file = %relative_path.display(), error = %error, "PDF extraction failed");
            ArtifactExtraction::failed("pdf_text", "PDF 正文读取失败，已保留原文件")
        }),
        "pdf" => ArtifactExtraction::indexed(json!({
            "status": "skipped",
            "reason": "PDF 文件超过首期文本抽取大小限制"
        })),
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "wmv" => {
            extract_video_metadata(path, ffprobe_bin).unwrap_or_else(|error| {
                tracing::warn!(file = %relative_path.display(), error = %error, "video metadata extraction failed");
                ArtifactExtraction::failed("ffprobe", "视频元数据暂不可用，已保留原文件")
            })
        }
        _ if should_extract_plain_text(relative_path, size_bytes) => {
            extract_plain_text(path, kind, &extension).unwrap_or_else(|error| {
                tracing::warn!(file = %relative_path.display(), error = %error, "plain text extraction failed");
                ArtifactExtraction::failed("text_preview", "文本预览读取失败，已保留原文件")
            })
        }
        _ => ArtifactExtraction::indexed(source_metadata(kind, &extension)),
    }
}

pub(super) fn generate_visual_preview(
    source_path: &Path,
    relative_path: &Path,
    kind: &str,
    preview_root: &Path,
    ffmpeg_bin: &str,
    pdftoppm_bin: &str,
) -> anyhow::Result<Option<GeneratedPreview>> {
    let extension = relative_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (tool, extractor) = if kind == "video" {
        (ffmpeg_bin, "ffmpeg_thumbnail")
    } else if extension == "pdf" {
        (pdftoppm_bin, "pdftoppm_first_page")
    } else {
        return Ok(None);
    };

    fs::create_dir_all(preview_root)?;
    let digest = format!(
        "{:x}",
        Sha256::digest(relative_path.to_string_lossy().as_bytes())
    );
    let output_path = preview_root.join(format!("preview-{}.jpg", &digest[..16]));
    let mut command = Command::new(tool);
    if kind == "video" {
        command.args(["-v", "error", "-ss", "0.5", "-i"]);
        command.arg(source_path);
        command.args([
            "-frames:v",
            "1",
            "-vf",
            "scale='min(1280,iw)':-2",
            "-q:v",
            "3",
            "-y",
        ]);
        command.arg(&output_path);
    } else {
        let output_prefix = output_path.with_extension("");
        command.args(["-f", "1", "-l", "1", "-singlefile", "-jpeg", "-r", "120"]);
        command.arg(source_path);
        command.arg(&output_prefix);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().context("preview tool is not available")?;
    let status = child.wait_timeout(PREVIEW_TIMEOUT)?;
    let Some(status) = status else {
        let _ = child.kill();
        let _ = child.wait();
        bail!("preview generation timed out");
    };
    if !status.success() || !output_path.is_file() {
        bail!("preview generation failed");
    }
    let size_bytes = fs::metadata(&output_path)?.len();
    if size_bytes == 0 {
        bail!("preview image is empty");
    }
    Ok(Some(GeneratedPreview {
        output_path,
        extractor,
        metadata: json!({
            "status": "ok",
            "sourcePath": relative_path.to_string_lossy(),
            "sizeBytes": size_bytes
        }),
    }))
}

fn extract_docx(path: &Path) -> anyhow::Result<ArtifactExtraction> {
    let mut archive = open_inner_archive(path)?;
    let mut parts = vec!["word/document.xml".to_owned()];
    parts.extend(
        archive
            .file_names()
            .filter(|name| {
                (name.starts_with("word/header") || name.starts_with("word/footer"))
                    && name.ends_with(".xml")
            })
            .map(str::to_owned),
    );
    parts.sort();
    parts.dedup();
    let mut text = String::new();
    for part in parts {
        if let Ok(xml) = read_zip_entry_limited(&mut archive, &part) {
            append_limited(&mut text, &extract_xml_text(&xml)?, MAX_TEXT_CHARS);
            if text.len() >= MAX_TEXT_CHARS {
                break;
            }
        }
    }
    let paragraphs = text.lines().filter(|line| !line.trim().is_empty()).count();
    Ok(ArtifactExtraction {
        extractor: "docx_text".to_owned(),
        text: non_empty(text.clone()),
        metadata: json!({
            "status": "ok",
            "paragraphCount": paragraphs,
            "textChars": text.chars().count()
        }),
    })
}

fn extract_pptx(path: &Path) -> anyhow::Result<ArtifactExtraction> {
    let mut archive = open_inner_archive(path)?;
    let mut slides = archive
        .file_names()
        .filter(|name| name.starts_with("ppt/slides/slide") && name.ends_with(".xml"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    slides.sort_by_key(|name| numeric_suffix(name));
    let mut notes = archive
        .file_names()
        .filter(|name| name.starts_with("ppt/notesSlides/notesSlide") && name.ends_with(".xml"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    notes.sort_by_key(|name| numeric_suffix(name));

    let mut text = String::new();
    let mut first_slide_text = String::new();
    for (index, slide) in slides.iter().enumerate() {
        let xml = read_zip_entry_limited(&mut archive, slide)?;
        let slide_text = extract_xml_text(&xml)?;
        if index == 0 {
            first_slide_text = slide_text.clone();
        }
        if !slide_text.trim().is_empty() {
            append_limited(
                &mut text,
                &format!("\n[第 {} 页]\n{}", index + 1, slide_text),
                MAX_TEXT_CHARS,
            );
        }
        if text.len() >= MAX_TEXT_CHARS {
            break;
        }
    }
    for note in &notes {
        if text.len() >= MAX_TEXT_CHARS {
            break;
        }
        let xml = read_zip_entry_limited(&mut archive, note)?;
        let note_text = extract_xml_text(&xml)?;
        if !note_text.trim().is_empty() {
            append_limited(
                &mut text,
                &format!("\n[演讲者备注]\n{note_text}"),
                MAX_TEXT_CHARS,
            );
        }
    }

    Ok(ArtifactExtraction {
        extractor: "pptx_text".to_owned(),
        text: non_empty(text.clone()),
        metadata: json!({
            "status": "ok",
            "slideCount": slides.len(),
            "notesCount": notes.len(),
            "firstSlideText": truncate_chars(first_slide_text.trim(), 500),
            "textChars": text.chars().count()
        }),
    })
}

fn extract_pdf(path: &Path) -> anyhow::Result<ArtifactExtraction> {
    let text = pdf_extract::extract_text(path).context("pdf text extraction failed")?;
    let text = truncate_chars(&text, MAX_TEXT_CHARS);
    Ok(ArtifactExtraction {
        extractor: "pdf_text".to_owned(),
        text: non_empty(text.clone()),
        metadata: json!({
            "status": "ok",
            "textChars": text.chars().count()
        }),
    })
}

fn extract_plain_text(
    path: &Path,
    kind: &str,
    extension: &str,
) -> anyhow::Result<ArtifactExtraction> {
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_PLAIN_TEXT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PLAIN_TEXT_BYTES {
        bytes.truncate(MAX_PLAIN_TEXT_BYTES as usize);
    }
    let text = truncate_chars(&String::from_utf8_lossy(&bytes), MAX_TEXT_CHARS);
    let mut metadata = source_metadata(kind, extension);
    if let Some(object) = metadata.as_object_mut() {
        object.insert("status".to_owned(), Value::String("ok".to_owned()));
        object.insert("textChars".to_owned(), json!(text.chars().count()));
    }
    Ok(ArtifactExtraction {
        extractor: if kind == "code" {
            "source_manifest".to_owned()
        } else {
            "text_preview".to_owned()
        },
        text: non_empty(text),
        metadata,
    })
}

fn extract_video_metadata(path: &Path, ffprobe_bin: &str) -> anyhow::Result<ArtifactExtraction> {
    let mut child = Command::new(ffprobe_bin)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration,size,format_name:stream=codec_type,codec_name,width,height",
            "-of",
            "json",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("ffprobe is not available")?;
    let status = child.wait_timeout(FFPROBE_TIMEOUT)?;
    let Some(status) = status else {
        let _ = child.kill();
        let _ = child.wait();
        bail!("ffprobe timed out");
    };
    if !status.success() {
        bail!("ffprobe rejected the video");
    }
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout)?;
    }
    let details: Value = serde_json::from_str(&stdout).context("invalid ffprobe output")?;
    Ok(ArtifactExtraction {
        extractor: "ffprobe".to_owned(),
        text: None,
        metadata: json!({
            "status": "ok",
            "probe": details
        }),
    })
}

fn open_inner_archive(path: &Path) -> anyhow::Result<ZipArchive<File>> {
    let archive = ZipArchive::new(File::open(path)?)?;
    if archive.len() > MAX_INNER_ARCHIVE_ENTRIES {
        bail!("office archive has too many entries");
    }
    Ok(archive)
}

fn read_zip_entry_limited<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> anyhow::Result<Vec<u8>> {
    let entry = archive.by_name(name)?;
    if entry.size() > MAX_INNER_XML_BYTES {
        bail!("office XML part is too large");
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry
        .take(MAX_INNER_XML_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INNER_XML_BYTES {
        bail!("office XML part is too large");
    }
    Ok(bytes)
}

fn extract_xml_text(xml: &[u8]) -> anyhow::Result<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut in_text = false;
    let mut output = String::new();
    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) => {
                in_text = event.local_name().as_ref() == b"t";
            }
            Event::Text(event) if in_text => {
                let value = event.decode()?;
                if !value.trim().is_empty() {
                    if !output.is_empty() && !output.ends_with([' ', '\n']) {
                        output.push(' ');
                    }
                    append_limited(&mut output, value.trim(), MAX_TEXT_CHARS);
                }
            }
            Event::End(event) => {
                let name = event.local_name();
                if name.as_ref() == b"t" {
                    in_text = false;
                } else if name.as_ref() == b"p" && !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            Event::Eof => break,
            _ => {}
        }
        if output.len() >= MAX_TEXT_CHARS {
            break;
        }
        buffer.clear();
    }
    Ok(output.trim().to_owned())
}

fn should_extract_plain_text(path: &Path, size_bytes: u64) -> bool {
    if size_bytes > MAX_PLAIN_TEXT_BYTES {
        return false;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.starts_with("readme")
        || name.starts_with("license")
        || matches!(
            name.as_str(),
            "package.json"
                | "cargo.toml"
                | "requirements.txt"
                | "pyproject.toml"
                | "pom.xml"
                | "build.gradle"
                | "dockerfile"
                | "makefile"
        )
        || matches!(
            extension.as_str(),
            "md" | "txt" | "toml" | "yaml" | "yml" | "json" | "tex"
        )
}

fn source_metadata(kind: &str, extension: &str) -> Value {
    if kind == "code" {
        json!({
            "language": language_for_extension(extension)
        })
    } else {
        json!({})
    }
}

fn language_for_extension(extension: &str) -> Option<&'static str> {
    match extension {
        "rs" => Some("Rust"),
        "ts" | "tsx" => Some("TypeScript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript"),
        "py" => Some("Python"),
        "java" => Some("Java"),
        "kt" | "kts" => Some("Kotlin"),
        "go" => Some("Go"),
        "c" | "h" => Some("C"),
        "cpp" | "hpp" => Some("C++"),
        "cs" => Some("C#"),
        "php" => Some("PHP"),
        "rb" => Some("Ruby"),
        "swift" => Some("Swift"),
        "dart" => Some("Dart"),
        "vue" => Some("Vue"),
        "svelte" => Some("Svelte"),
        "html" => Some("HTML"),
        "css" | "scss" => Some("CSS"),
        "sql" => Some("SQL"),
        "ino" => Some("Arduino"),
        _ => None,
    }
}

fn numeric_suffix(value: &str) -> usize {
    value
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(usize::MAX)
}

fn append_limited(target: &mut String, value: &str, max_chars: usize) {
    let remaining = max_chars.saturating_sub(target.chars().count());
    target.extend(value.chars().take(remaining));
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::{extract_xml_text, language_for_extension, numeric_suffix};

    #[test]
    fn extracts_text_from_ooxml_namespaces() {
        let xml = r#"<w:document xmlns:w="urn:w"><w:body><w:p><w:r><w:t>项目名称</w:t></w:r><w:r><w:t>测试平台</w:t></w:r></w:p></w:body></w:document>"#;
        let text = extract_xml_text(xml.as_bytes()).expect("OOXML text");
        assert_eq!(text, "项目名称 测试平台");
    }

    #[test]
    fn sorts_numbered_office_parts_numerically() {
        assert!(numeric_suffix("ppt/slides/slide2.xml") < numeric_suffix("ppt/slides/slide10.xml"));
    }

    #[test]
    fn identifies_common_source_languages() {
        assert_eq!(language_for_extension("rs"), Some("Rust"));
        assert_eq!(language_for_extension("tsx"), Some("TypeScript"));
        assert_eq!(language_for_extension("bin"), None);
    }
}
