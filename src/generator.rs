use crate::credentials::get_credential_internal;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

const CLAUDE_MODEL: &str = "claude-sonnet-4-6";
const CLAUDE_TIMEOUT_SECS: u64 = 60;

// ── Shared types ──────────────────────────────────────────────────────────────
// IssuePayload is the shared contract between generate_feature_request (here),
// the frontend preview editor, and create_github_issues (github.rs).
// Field changes must be coordinated across all three.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureBrief {
    pub feature_name: String,
    pub summary: String,
    pub problem: String,
    pub goals: Vec<String>,
    pub non_goals: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssuePayload {
    pub title: String,
    pub body: String,
    pub area: String,
    pub acceptance_criteria: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationOutput {
    pub brief: FeatureBrief,
    pub issues: Vec<IssuePayload>,
}

// ── Prompt ────────────────────────────────────────────────────────────────────

fn build_system_prompt(owner: &str, repo_name: &str) -> String {
    format!(
        r#"You are a senior software architect generating GitHub Issues for {owner}/{repo_name}.

The user will describe a rough feature idea. Your job is to:
1. Write a concise feature brief
2. Break it into 4–8 actionable implementation tickets ordered by dependency

CRITICAL: Return ONLY a valid JSON object. No markdown, no code fences, no text before or after.

Required schema:
{{
  "brief": {{
    "feature_name": "string",
    "summary": "string — 1–2 sentences",
    "problem": "string — what problem this solves",
    "goals": ["string"],
    "non_goals": ["string"]
  }},
  "issues": [
    {{
      "title": "string — action-oriented, 50–80 chars",
      "body": "string — markdown, ≥3 paragraphs covering what/why/how, self-contained",
      "area": "Backend | Frontend | Database | Integration | Testing | Infrastructure",
      "acceptance_criteria": ["string — specific and testable"],
      "dependencies": ["title of prerequisite issue, or empty array"]
    }}
  ]
}}

Rules:
- Order issues from foundational (no deps) to dependent
- acceptance_criteria must be concrete and verifiable
- Each body must be detailed enough for another engineer or AI to implement without follow-up
- Target repo: {owner}/{repo_name}"#,
        owner = owner,
        repo_name = repo_name,
    )
}

// ── Command ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_feature_request(
    owner: String,
    repo_name: String,
    idea: String,
) -> Result<GenerationOutput, String> {
    let api_key = get_credential_internal("anthropic_key")?
        .ok_or("Anthropic API key not set — open Settings and save your key.")?;

    if api_key.trim().is_empty() {
        return Err("Anthropic API key is empty — open Settings and save your key.".into());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(CLAUDE_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let body = json!({
        "model": CLAUDE_MODEL,
        "max_tokens": 8192,
        "system": build_system_prompt(&owner, &repo_name),
        "messages": [{ "role": "user", "content": idea }]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Claude API timed out — please try again.".to_string()
            } else {
                format!("Claude API request failed: {}", e)
            }
        })?;

    let status = response.status();
    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Claude response: {}", e))?;

    if !status.is_success() {
        let msg = data["error"]["message"]
            .as_str()
            .unwrap_or("Unknown Claude API error");
        return Err(format!("Claude API error {}: {}", status, msg));
    }

    let raw = data["content"][0]["text"]
        .as_str()
        .ok_or("No text content in Claude response")?;

    // Strip any accidental markdown fences
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str::<GenerationOutput>(cleaned).map_err(|e| {
        format!(
            "Failed to parse generation output ({}). Raw: {}",
            e,
            &cleaned.chars().take(500).collect::<String>()
        )
    })
}
