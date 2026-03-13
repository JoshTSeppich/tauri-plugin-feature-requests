# tauri-plugin-feature-requests

A Tauri v2 plugin that adds an in-app **Feature Request Generator** to any Tauri desktop app.

Users describe a feature idea in plain text → Claude AI generates a structured feature brief + implementation tickets → user reviews and edits inline → issues published directly to a GitHub repository.

---

## What it provides

Seven Tauri commands, all available on the frontend via `invoke("plugin:feature-requests|<command>")`:

| Command | Description |
|---|---|
| `save_credential` | Persist a credential to the OS keychain |
| `get_credential` | Read a credential from the OS keychain |
| `list_saved_repos` | Return saved GitHub repos (SQLite, sorted newest first) |
| `upsert_saved_repo` | Add or update a repo entry |
| `delete_saved_repo` | Remove a repo entry by ID |
| `generate_feature_request` | Call Claude → return `{ brief, issues[] }` as structured JSON |
| `create_github_issues` | POST each issue to GitHub REST API, return per-issue results |

---

## Integration — 3 steps

### 1. Add the dependency

In your Tauri app's `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-feature-requests = { path = "/path/to/tauri-plugin-feature-requests" }
# or once published:
# tauri-plugin-feature-requests = "0.1"
```

### 2. Register the plugin

In `src-tauri/src/lib.rs` (or `main.rs`):

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_feature_requests::init())  // ← one line
        // ... rest of your setup
        .invoke_handler(tauri::generate_handler![/* your other commands */])
        .run(tauri::generate_context!())
        .expect("error while running app");
}
```

That's it on the Rust side. The plugin self-manages its SQLite database (stored in the OS app data directory alongside your existing DB).

### 3. Call from the frontend

All commands are namespaced under `plugin:feature-requests`:

```js
import { invoke } from "@tauri-apps/api/core";

// Save a credential
await invoke("plugin:feature-requests|save_credential", {
  key: "anthropic_key",
  value: "sk-ant-..."
});

// Generate issues from a feature idea
const output = await invoke("plugin:feature-requests|generate_feature_request", {
  owner: "foxworks",
  repoName: "eventfold-crm",
  idea: "Add a dark mode toggle that persists between sessions"
});
// output: { brief: { feature_name, summary, problem, goals, non_goals },
//            issues: [{ title, body, area, acceptance_criteria, dependencies }] }

// Publish to GitHub
const results = await invoke("plugin:feature-requests|create_github_issues", {
  owner: "foxworks",
  repoName: "eventfold-crm",
  issues: output.issues  // or user-edited version
});
// results: [{ title, status: "success"|"error", url?, error? }]
```

---

## Credentials

Stored in the **OS keychain** (macOS Keychain, Windows Credential Manager, Linux Secret Service) under service `"eventfold"`.

| Key | Description |
|---|---|
| `anthropic_key` | Anthropic API key (`sk-ant-...`) |
| `github_pat` | GitHub Personal Access Token — requires `repo` scope (`issues:write`) |

---

## Data model

### `GenerationOutput`
```
{
  brief: {
    feature_name: string
    summary:      string
    problem:      string
    goals:        string[]
    non_goals:    string[]
  },
  issues: IssuePayload[]
}
```

### `IssuePayload`
```
{
  title:                string
  body:                 string   // markdown
  area:                 string   // "Backend" | "Frontend" | ...
  acceptance_criteria:  string[]
  dependencies:         string[] // titles of prerequisite issues
}
```

### `IssueResult`
```
{
  title:  string
  status: "success" | "error"
  url?:   string   // GitHub html_url on success
  error?: string   // GitHub error message on failure
}
```

---

## Platform support

| Platform | Keychain backend | Tested |
|---|---|---|
| macOS | Keychain Access | ✓ |
| Windows | Credential Manager | — |
| Linux | Secret Service (libsecret) | — |

---

## License

MIT
