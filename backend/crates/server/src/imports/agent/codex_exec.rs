use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{bail, Context};
use async_trait::async_trait;
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::Command,
};

use super::{
    agent_prompt, bounded_timeout, parse_agent_result, AgentRunOutcome, AgentRunRequest,
    ImportAgentRunner, OUTPUT_SCHEMA,
};

const MAX_STDERR_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct CodexExecConfig {
    pub enabled: bool,
    pub binary: PathBuf,
    pub codex_home: PathBuf,
    pub runtime_root: PathBuf,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key_file: Option<PathBuf>,
    pub timeout: Duration,
}

#[derive(Clone, Debug)]
pub(crate) struct CodexExecRunner {
    config: CodexExecConfig,
}

impl CodexExecRunner {
    pub fn new(mut config: CodexExecConfig) -> anyhow::Result<Self> {
        config.timeout = bounded_timeout(config.timeout.as_secs());
        let startup_dir =
            std::env::current_dir().context("failed to resolve process working directory")?;
        if config.binary.is_relative() && config.binary.components().count() > 1 {
            config.binary = startup_dir.join(&config.binary);
        }
        if config.codex_home.is_relative() {
            config.codex_home = startup_dir.join(&config.codex_home);
        }
        if config.runtime_root.is_relative() {
            config.runtime_root = startup_dir.join(&config.runtime_root);
        }
        if let Some(api_key_file) = config.api_key_file.as_mut() {
            if api_key_file.is_relative() {
                *api_key_file = startup_dir.join(&*api_key_file);
            }
        }
        if config.enabled {
            if config.base_url.as_deref().is_none_or(str::is_empty)
                || config.model.as_deref().is_none_or(str::is_empty)
            {
                bail!("enabled Codex runner is missing base URL or model");
            }
            if config.api_key_file.is_none() && !config.codex_home.join("auth.json").is_file() {
                bail!("enabled Codex runner requires an API key file or CODEX_HOME/auth.json");
            }
            let base_url = config.base_url.as_deref().expect("checked above");
            let url = reqwest::Url::parse(base_url).context("invalid Codex base URL")?;
            if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
                bail!("Codex base URL must be an absolute HTTP(S) URL");
            }
        }
        Ok(Self { config })
    }

    #[cfg(test)]
    pub fn disabled() -> Self {
        Self {
            config: CodexExecConfig {
                enabled: false,
                binary: PathBuf::from("codex"),
                codex_home: PathBuf::from("data/codex-home"),
                runtime_root: PathBuf::from("data/codex-runs"),
                base_url: None,
                model: None,
                api_key_file: None,
                timeout: Duration::from_secs(600),
            },
        }
    }

    fn command_args(
        &self,
        request: &AgentRunRequest,
        schema_path: &Path,
        result_path: &Path,
    ) -> anyhow::Result<Vec<String>> {
        let base_url = self
            .config
            .base_url
            .as_deref()
            .context("Codex base URL is not configured")?;
        let model = self
            .config
            .model
            .as_deref()
            .context("Codex model is not configured")?;
        Ok(vec![
            "exec".to_owned(),
            "--json".to_owned(),
            "--ephemeral".to_owned(),
            "--ignore-user-config".to_owned(),
            "--ignore-rules".to_owned(),
            "--skip-git-repo-check".to_owned(),
            "--sandbox".to_owned(),
            "read-only".to_owned(),
            "--cd".to_owned(),
            request.analysis_dir.to_string_lossy().into_owned(),
            "--model".to_owned(),
            model.to_owned(),
            "--output-schema".to_owned(),
            schema_path.to_string_lossy().into_owned(),
            "--output-last-message".to_owned(),
            result_path.to_string_lossy().into_owned(),
            "--config".to_owned(),
            format!("openai_base_url={}", serde_json::to_string(base_url)?),
            "--config".to_owned(),
            "shell_environment_policy.inherit=\"core\"".to_owned(),
            "--config".to_owned(),
            "shell_environment_policy.ignore_default_excludes=false".to_owned(),
            "-".to_owned(),
        ])
    }

    async fn prepare_runtime_home(&self, run_id: &str) -> anyhow::Result<tempfile::TempDir> {
        tokio::fs::create_dir_all(&self.config.runtime_root)
            .await
            .with_context(|| {
                format!(
                    "failed to create Codex runtime root {}",
                    self.config.runtime_root.display()
                )
            })?;
        let runtime_home = tempfile::Builder::new()
            .prefix(&format!("codex-{run_id}-"))
            .tempdir_in(&self.config.runtime_root)
            .with_context(|| {
                format!(
                    "failed to create a writable Codex runtime home in {}",
                    self.config.runtime_root.display()
                )
            })?;

        copy_codex_home_file(
            &self.config.codex_home,
            runtime_home.path(),
            "config.toml",
            false,
        )
        .await?;
        copy_codex_home_file(
            &self.config.codex_home,
            runtime_home.path(),
            "auth.json",
            self.config.api_key_file.is_none(),
        )
        .await?;
        Ok(runtime_home)
    }
}

#[async_trait]
impl ImportAgentRunner for CodexExecRunner {
    fn enabled(&self) -> bool {
        self.config.enabled
    }

    fn runner_name(&self) -> &'static str {
        "codex_exec"
    }

    fn model_name(&self) -> Option<&str> {
        self.config.model.as_deref()
    }

    fn base_url_origin(&self) -> Option<String> {
        let url = reqwest::Url::parse(self.config.base_url.as_deref()?).ok()?;
        let host = url.host_str()?;
        Some(match url.port() {
            Some(port) => format!("{}://{}:{port}", url.scheme(), host),
            None => format!("{}://{}", url.scheme(), host),
        })
    }

    async fn run(&self, mut request: AgentRunRequest) -> anyhow::Result<AgentRunOutcome> {
        if !self.enabled() {
            bail!("Codex runner is disabled");
        }
        let process_dir = std::env::current_dir()
            .context("failed to resolve the Codex worker process directory")?;
        request.analysis_dir = absolute_from(&process_dir, &request.analysis_dir);
        tokio::fs::create_dir_all(&request.analysis_dir).await?;
        request.analysis_dir = tokio::fs::canonicalize(&request.analysis_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to resolve the Codex analysis directory {}",
                    request.analysis_dir.display()
                )
            })?;
        let runtime_home = self.prepare_runtime_home(&request.run_id).await?;

        let api_key = if let Some(api_key_file) = self.config.api_key_file.as_deref() {
            let value = tokio::fs::read_to_string(api_key_file)
                .await
                .with_context(|| {
                    format!(
                        "failed to read Codex API key file {}",
                        api_key_file.display()
                    )
                })?;
            let value = value.trim().to_owned();
            if value.is_empty() {
                bail!("Codex API key file is empty");
            }
            Some(value)
        } else {
            None
        };

        let schema_path = request
            .analysis_dir
            .join(format!("agent-schema-{}.json", request.run_id));
        let result_path = request
            .analysis_dir
            .join(format!("agent-result-{}.json", request.run_id));
        let raw_events_path = request
            .analysis_dir
            .join(format!("agent-events-{}.jsonl", request.run_id));
        tokio::fs::write(&schema_path, OUTPUT_SCHEMA).await?;

        let mut command = Command::new(&self.config.binary);
        command
            .args(self.command_args(&request, &schema_path, &result_path)?)
            .env("CODEX_HOME", runtime_home.path())
            .env_remove("GH_TOKEN")
            .env_remove("GITHUB_TOKEN")
            .current_dir(&request.analysis_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(api_key) = api_key.as_deref() {
            command.env("CODEX_API_KEY", api_key);
        }
        let mut child = command.spawn().with_context(|| {
            format!(
                "failed to start pinned Codex binary {}",
                self.config.binary.display()
            )
        })?;

        let prompt = agent_prompt(&request);
        let mut stdin = child
            .stdin
            .take()
            .context("Codex stdin was not available")?;
        stdin.write_all(prompt.as_bytes()).await?;
        stdin.shutdown().await?;

        let stdout = child
            .stdout
            .take()
            .context("Codex stdout was not available")?;
        let stderr = child
            .stderr
            .take()
            .context("Codex stderr was not available")?;
        let events_path_for_task = raw_events_path.clone();
        let event_task = tokio::spawn(read_jsonl_events(stdout, events_path_for_task));
        let stderr_task = tokio::spawn(read_bounded(stderr, MAX_STDERR_BYTES));

        let status = match tokio::time::timeout(self.config.timeout, child.wait()).await {
            Ok(status) => status.context("failed while waiting for Codex")?,
            Err(_) => {
                let _ = child.kill().await;
                bail!(
                    "Codex timed out after {} seconds",
                    self.config.timeout.as_secs()
                );
            }
        };
        let event_summary = event_task.await.context("Codex event reader stopped")??;
        let stderr = stderr_task.await.context("Codex stderr reader stopped")??;
        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr);
            bail!("Codex exited with status {}: {}", status, stderr.trim());
        }

        let result_bytes = tokio::fs::read(&result_path)
            .await
            .context("Codex did not write its structured final output")?;
        let result = parse_agent_result(&result_bytes)?;
        Ok(AgentRunOutcome {
            thread_id: event_summary.thread_id,
            result,
            raw_events_path,
        })
    }
}

fn absolute_from(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

async fn copy_codex_home_file(
    source_home: &Path,
    runtime_home: &Path,
    file_name: &str,
    required: bool,
) -> anyhow::Result<()> {
    let source = source_home.join(file_name);
    if !source.is_file() {
        if required {
            bail!("required Codex home file is missing: {}", source.display());
        }
        return Ok(());
    }

    let destination = runtime_home.join(file_name);
    tokio::fs::copy(&source, &destination)
        .await
        .with_context(|| {
            format!(
                "failed to copy Codex home file {} into the writable runtime home",
                source.display()
            )
        })?;
    set_owner_only_permissions(&destination).await?;
    Ok(())
}

#[cfg(unix)]
async fn set_owner_only_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .await
        .with_context(|| format!("failed to secure copied Codex file {}", path.display()))
}

#[cfg(not(unix))]
async fn set_owner_only_permissions(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[derive(Default)]
struct EventSummary {
    thread_id: Option<String>,
}

async fn read_jsonl_events(
    stdout: impl AsyncRead + Unpin,
    destination: PathBuf,
) -> anyhow::Result<EventSummary> {
    let mut reader = BufReader::new(stdout).lines();
    let mut output = tokio::fs::File::create(destination).await?;
    let mut summary = EventSummary::default();
    while let Some(line) = reader.next_line().await? {
        output.write_all(line.as_bytes()).await?;
        output.write_all(b"\n").await?;
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if value.get("type").and_then(Value::as_str) == Some("thread.started") {
                summary.thread_id = value
                    .get("thread_id")
                    .or_else(|| value.get("threadId"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
        }
    }
    output.flush().await?;
    Ok(summary)
}

async fn read_bounded(reader: impl AsyncRead + Unpin, limit: usize) -> anyhow::Result<Vec<u8>> {
    let mut reader = BufReader::new(reader);
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = tokio::io::AsyncReadExt::read(&mut reader, &mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(output.len());
        if remaining > 0 {
            output.extend_from_slice(&buffer[..read.min(remaining)]);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_isolated_and_does_not_embed_the_api_key() {
        let runner = CodexExecRunner::new(CodexExecConfig {
            enabled: true,
            binary: PathBuf::from("vendor/codex/codex"),
            codex_home: PathBuf::from("data/codex-home"),
            runtime_root: PathBuf::from("data/codex-runs"),
            base_url: Some("https://api.example.test/v1".to_owned()),
            model: Some("configured-model".to_owned()),
            api_key_file: Some(PathBuf::from("/run/secrets/codex-api-key")),
            timeout: Duration::from_secs(600),
        })
        .expect("runner");
        let request = AgentRunRequest {
            run_id: "run-1".to_owned(),
            job_id: "job-1".to_owned(),
            analysis_dir: PathBuf::from("analysis"),
            analysis_bundle_path: PathBuf::from("analysis-bundle.json"),
            refinement_prompt: String::new(),
        };
        let args = runner
            .command_args(
                &request,
                Path::new("analysis/schema.json"),
                Path::new("analysis/result.json"),
            )
            .expect("args");
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--sandbox", "read-only"]));
        assert!(args.contains(&"--ignore-user-config".to_owned()));
        assert!(args.contains(&"--ignore-rules".to_owned()));
        assert!(args.contains(&"shell_environment_policy.ignore_default_excludes=false".to_owned()));
        assert!(!args.join(" ").contains("codex-api-key"));
    }

    #[test]
    fn runner_accepts_native_codex_home_auth() {
        let directory = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            directory.path().join("auth.json"),
            r#"{"OPENAI_API_KEY":"not-used-by-this-test"}"#,
        )
        .expect("auth file");
        let runner = CodexExecRunner::new(CodexExecConfig {
            enabled: true,
            binary: PathBuf::from("codex"),
            codex_home: directory.path().to_path_buf(),
            runtime_root: directory.path().join("runs"),
            base_url: Some("https://api.example.test/v1".to_owned()),
            model: Some("configured-model".to_owned()),
            api_key_file: None,
            timeout: Duration::from_secs(600),
        });
        assert!(runner.is_ok());
    }

    #[test]
    fn runner_resolves_relative_executable_before_changing_analysis_directory() {
        let directory = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            directory.path().join("auth.json"),
            r#"{"OPENAI_API_KEY":"not-used-by-this-test"}"#,
        )
        .expect("auth file");
        let runner = CodexExecRunner::new(CodexExecConfig {
            enabled: true,
            binary: PathBuf::from("tools/codex"),
            codex_home: directory.path().to_path_buf(),
            runtime_root: directory.path().join("runs"),
            base_url: Some("https://api.example.test/v1".to_owned()),
            model: Some("configured-model".to_owned()),
            api_key_file: None,
            timeout: Duration::from_secs(600),
        })
        .expect("runner");

        assert!(runner.config.binary.is_absolute());
        assert!(runner.config.binary.ends_with(Path::new("tools/codex")));
    }

    #[test]
    fn relative_analysis_paths_are_resolved_before_the_child_changes_directory() {
        let base = Path::new("/srv/icthub/backend");
        let relative = Path::new("uploads/imports/job-1/analysis");
        assert_eq!(
            absolute_from(base, relative),
            base.join("uploads/imports/job-1/analysis")
        );

        let absolute = base.join("uploads/imports/job-2/analysis");
        assert_eq!(absolute_from(base, &absolute), absolute);
    }

    #[tokio::test]
    async fn runtime_home_copies_only_codex_credentials_and_is_removed_on_drop() {
        let directory = tempfile::tempdir().expect("tempdir");
        let source_home = directory.path().join("source-home");
        let runtime_root = directory.path().join("runs");
        std::fs::create_dir_all(&source_home).expect("source home");
        std::fs::write(source_home.join("config.toml"), "model = \"test\"").expect("config file");
        std::fs::write(source_home.join("auth.json"), r#"{"token":"secret"}"#).expect("auth file");
        std::fs::write(source_home.join("history.jsonl"), "must not be copied")
            .expect("unrelated file");
        let runner = CodexExecRunner::new(CodexExecConfig {
            enabled: true,
            binary: PathBuf::from("codex"),
            codex_home: source_home,
            runtime_root,
            base_url: Some("https://api.example.test/v1".to_owned()),
            model: Some("configured-model".to_owned()),
            api_key_file: None,
            timeout: Duration::from_secs(600),
        })
        .expect("runner");

        let runtime_home = runner
            .prepare_runtime_home("run-1")
            .await
            .expect("runtime home");
        let runtime_path = runtime_home.path().to_path_buf();
        assert_eq!(
            std::fs::read_to_string(runtime_path.join("config.toml")).expect("copied config"),
            "model = \"test\""
        );
        assert!(runtime_path.join("auth.json").is_file());
        assert!(!runtime_path.join("history.jsonl").exists());

        drop(runtime_home);
        assert!(!runtime_path.exists());
    }

    #[tokio::test]
    async fn jsonl_reader_captures_thread_id_and_raw_events() {
        let directory = tempfile::tempdir().expect("tempdir");
        let destination = directory.path().join("events.jsonl");
        let input = b"{\"type\":\"thread.started\",\"thread_id\":\"thread-123\"}\n{\"type\":\"turn.completed\"}\n";
        let summary = read_jsonl_events(&input[..], destination.clone())
            .await
            .expect("events");
        assert_eq!(summary.thread_id.as_deref(), Some("thread-123"));
        assert_eq!(std::fs::read(destination).expect("raw events"), input);
    }
}
