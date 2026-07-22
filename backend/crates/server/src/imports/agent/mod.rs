mod codex_exec;

use std::{path::PathBuf, time::Duration};

use anyhow::{bail, Context};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub(crate) use codex_exec::{CodexExecConfig, CodexExecRunner};

pub(crate) const OUTPUT_SCHEMA: &str =
    include_str!("../../../../../schemas/import-agent-result.schema.json");
pub(crate) const SYSTEM_PROMPT: &str = include_str!("../../../../../prompts/import-project.md");

pub(crate) const PRIMARY_CATEGORIES: [&str; 5] =
    ["传统软件", "智能硬件", "AI 软件", "数字媒体", "研究成果"];

#[derive(Clone, Debug)]
pub(crate) struct AgentRunRequest {
    pub run_id: String,
    pub job_id: String,
    pub analysis_dir: PathBuf,
    pub analysis_bundle_path: PathBuf,
    pub refinement_prompt: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AgentRunOutcome {
    pub thread_id: Option<String>,
    pub result: AgentImportResult,
    pub raw_events_path: PathBuf,
}

#[async_trait]
pub(crate) trait ImportAgentRunner: Send + Sync {
    fn enabled(&self) -> bool;
    fn runner_name(&self) -> &'static str;
    fn model_name(&self) -> Option<&str>;
    fn base_url_origin(&self) -> Option<String>;

    async fn run(&self, request: AgentRunRequest) -> anyhow::Result<AgentRunOutcome>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentImportResult {
    pub project_name: String,
    pub summary: String,
    pub primary_category: String,
    pub suggested_tags: Vec<EvidencedValue>,
    pub owner: Option<EvidencedValue>,
    pub source: Option<EvidencedValue>,
    pub highest_award: Option<EvidencedValue>,
    pub resources: AgentNormalizedResources,
    pub warnings: Vec<String>,
}

impl AgentImportResult {
    pub fn validate(&self) -> anyhow::Result<()> {
        let project_name = self.project_name.trim();
        if project_name.is_empty() || project_name.chars().count() > 120 {
            bail!("Codex returned an invalid project name");
        }
        if self.summary.trim().is_empty() || self.summary.chars().count() > 500 {
            bail!("Codex returned an invalid project summary");
        }
        if !PRIMARY_CATEGORIES.contains(&self.primary_category.as_str()) {
            bail!("Codex returned an unsupported primary category");
        }
        for value in self
            .suggested_tags
            .iter()
            .chain(self.owner.iter())
            .chain(self.source.iter())
            .chain(self.highest_award.iter())
        {
            value.validate()?;
        }
        for resource in self.resources.iter() {
            resource.validate()?;
        }
        if self.suggested_tags.len() > 12 {
            bail!("Codex returned too many suggested tags");
        }
        if self.warnings.len() > 30 {
            bail!("Codex returned too many warnings");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidencedValue {
    pub value: String,
    pub evidence: String,
}

impl EvidencedValue {
    fn validate(&self) -> anyhow::Result<()> {
        if self.value.trim().is_empty() || self.value.chars().count() > 120 {
            bail!("Codex returned an invalid evidenced value");
        }
        if self.evidence.trim().is_empty() || self.evidence.chars().count() > 500 {
            bail!("Codex returned a value without concise evidence");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentNormalizedResources {
    pub source_code: Vec<AgentResource>,
    pub documents: Vec<AgentResource>,
    pub presentations: Vec<AgentResource>,
    pub videos: Vec<AgentResource>,
    pub links: Vec<AgentResource>,
}

impl AgentNormalizedResources {
    fn iter(&self) -> impl Iterator<Item = &AgentResource> {
        self.source_code
            .iter()
            .chain(self.documents.iter())
            .chain(self.presentations.iter())
            .chain(self.videos.iter())
            .chain(self.links.iter())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentResource {
    pub display_name: String,
    pub source_ref: String,
    pub evidence: String,
    pub confidence: f64,
}

impl AgentResource {
    fn validate(&self) -> anyhow::Result<()> {
        if self.display_name.trim().is_empty() || self.display_name.chars().count() > 180 {
            bail!("Codex returned an invalid resource name");
        }
        if self.source_ref.trim().is_empty() || self.source_ref.chars().count() > 2_048 {
            bail!("Codex returned an invalid resource reference");
        }
        if self.evidence.trim().is_empty() || self.evidence.chars().count() > 500 {
            bail!("Codex returned a resource without concise evidence");
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            bail!("Codex returned an invalid resource confidence");
        }
        Ok(())
    }
}

pub(crate) fn parse_agent_result(bytes: &[u8]) -> anyhow::Result<AgentImportResult> {
    let result: AgentImportResult =
        serde_json::from_slice(bytes).context("Codex final output is not valid JSON")?;
    result.validate()?;
    Ok(result)
}

pub(crate) fn agent_prompt(request: &AgentRunRequest) -> String {
    format!(
        "{SYSTEM_PROMPT}\n\n本次任务信息：\n- job_id: {}\n- 分析包文件: {}\n\n成员在材料整理完成后补充的提示：\n{}",
        request.job_id,
        request.analysis_bundle_path.display(),
        if request.refinement_prompt.trim().is_empty() {
            "（无补充提示）"
        } else {
            request.refinement_prompt.trim()
        }
    )
}

pub(crate) fn bounded_timeout(seconds: u64) -> Duration {
    Duration::from_secs(seconds.clamp(30, 3_600))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_rejects_unknown_primary_category() {
        let mut result = sample_result();
        result.primary_category = "互联网+".to_owned();
        assert!(result.validate().is_err());
    }

    #[test]
    fn result_requires_evidence_for_cautious_fields() {
        let mut result = sample_result();
        result.owner = Some(EvidencedValue {
            value: "张三".to_owned(),
            evidence: String::new(),
        });
        assert!(result.validate().is_err());
    }

    #[test]
    fn embedded_output_schema_is_valid_json() {
        let schema: serde_json::Value = serde_json::from_str(OUTPUT_SCHEMA).expect("schema JSON");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["primaryCategory"]["enum"]
                .as_array()
                .expect("category enum")
                .len(),
            5
        );
    }

    fn sample_result() -> AgentImportResult {
        AgentImportResult {
            project_name: "棉田智检".to_owned(),
            summary: "面向棉花病虫害的移动巡检平台。".to_owned(),
            primary_category: "AI 软件".to_owned(),
            suggested_tags: Vec::new(),
            owner: None,
            source: None,
            highest_award: None,
            resources: AgentNormalizedResources::default(),
            warnings: Vec::new(),
        }
    }
}
