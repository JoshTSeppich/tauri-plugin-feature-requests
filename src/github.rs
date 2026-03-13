use crate::credentials::get_credential_internal;
use crate::generator::IssuePayload;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueResult {
    pub title: String,
    /// `"success"` or `"error"`
    pub status: String,
    /// GitHub `html_url` for successfully created issues
    pub url: Option<String>,
    /// Human-readable error for failed issues
    pub error: Option<String>,
}

// ── Command ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_github_issues(
    owner: String,
    repo_name: String,
    issues: Vec<IssuePayload>,
) -> Result<Vec<IssueResult>, String> {
    // Retrieve PAT before touching the API — avoids partial state if missing
    let token = get_credential_internal("github_pat")?
        .ok_or("GitHub PAT not set — open Settings and save your token.")?;

    if token.trim().is_empty() {
        return Err("GitHub PAT is empty — open Settings and save your token.".into());
    }

    let client = Client::new();
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/issues",
        owner, repo_name
    );
    let mut results: Vec<IssueResult> = Vec::with_capacity(issues.len());

    // Sequential creation preserves issue ordering and avoids burst rate-limit hits
    for issue in &issues {
        let body = json!({ "title": issue.title, "body": issue.body });

        let response = client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "eventfold-crm")
            .header("Content-Type", "application/json")
            .header("Accept", "application/vnd.github+json")
            .json(&body)
            .send()
            .await;

        match response {
            Err(e) => results.push(IssueResult {
                title: issue.title.clone(),
                status: "error".into(),
                url: None,
                error: Some(format!("Network error: {}", e)),
            }),
            Ok(resp) => {
                let status = resp.status();
                let data: Value = resp.json().await.unwrap_or(json!({}));

                if status == 201 {
                    results.push(IssueResult {
                        title: issue.title.clone(),
                        status: "success".into(),
                        url: data["html_url"].as_str().map(|s| s.to_string()),
                        error: None,
                    });
                } else {
                    let err_msg = match status.as_u16() {
                        401 => "Authentication failed — check your GitHub PAT is valid".into(),
                        403 => "Permission denied — ensure your PAT has 'repo' scope (issues:write)".into(),
                        404 => format!("Repository '{}/{}' not found or PAT lacks access", owner, repo_name),
                        422 => {
                            let msg = data["message"].as_str().unwrap_or("Validation failed");
                            format!("GitHub rejected issue: {}", msg)
                        }
                        _ => {
                            let msg = data["message"].as_str().unwrap_or("Unknown error");
                            format!("GitHub error {}: {}", status, msg)
                        }
                    };
                    results.push(IssueResult {
                        title: issue.title.clone(),
                        status: "error".into(),
                        url: None,
                        error: Some(err_msg),
                    });
                }
            }
        }
    }

    Ok(results)
}
