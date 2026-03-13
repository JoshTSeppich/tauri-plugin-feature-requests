# tauri-plugin-feature-requests — Claude Code Context

## What this repo is

A **Tauri v2 plugin** that adds an in-app Feature Request Generator to any Tauri desktop app (built for EventFold CRM). Users describe a feature idea in plain text → Claude AI generates a structured feature brief + implementation tickets → user reviews/edits inline → issues published directly to GitHub.

The plugin exposes 7 Tauri commands to the frontend via `invoke("plugin:feature-requests|<command>")`.

## File map

```
src/
  lib.rs            — Plugin entry point. init<R>() → TauriPlugin<R>.
                      Sets up SQLite DB path, runs init_db(), manages DbConn state,
                      registers all 7 commands in generate_handler![].

  credentials.rs    — OS keychain (keyring = "3").
                      Service name: "eventfold". Keys: "anthropic_key", "github_pat".
                      save_credential / get_credential — Tauri commands (frontend-facing).
                      get_credential_internal(key: &str) — plain fn used by generator + github.

  repos.rs          — SQLite saved repository list.
                      DbConn = Mutex<Connection> (managed state).
                      Table: saved_repos(id, owner, repo_name, display_label, created_at,
                             UNIQUE(owner, repo_name)).
                      Commands: list_saved_repos, upsert_saved_repo, delete_saved_repo.
                      IMPORTANT: list_saved_repos uses explicit for-loop to materialize rows
                      before stmt/conn drop (rusqlite lifetime constraint).

  generator.rs      — Anthropic Messages API integration.
                      Model: claude-sonnet-4-6 (CLAUDE_MODEL const).
                      Timeout: 60s. max_tokens: 8192.
                      Structs: FeatureBrief, IssuePayload (shared with github.rs), GenerationOutput.
                      Command: generate_feature_request(owner, repo_name, idea).
                      Reads anthropic_key from keychain. Returns GenerationOutput or Err(String).

  github.rs         — GitHub REST API v3 issue creation.
                      Reads github_pat from keychain before any API calls.
                      Sequential loop — does NOT abort on first failure.
                      Struct: IssueResult { title, status, url?, error? }.
                      Command: create_github_issues(owner, repo_name, issues: Vec<IssuePayload>).
                      Status 201 = success. 401/403/404/422 have typed error messages.
```

## Key dependencies

| Crate | Version | Purpose |
|---|---|---|
| tauri | 2 | Plugin infrastructure |
| keyring | 3 | OS keychain (macOS Keychain / Win Credential Manager) |
| rusqlite | 0.32 + bundled | SQLite (statically linked, no system sqlite3 needed) |
| reqwest | 0.12, rustls-tls | HTTP for Anthropic + GitHub APIs |
| serde / serde_json | 1 | JSON serialization |
| tokio | 1 full | Async runtime for reqwest |

## Conventions

- **No external state** — the plugin is self-contained. DB path comes from `app.path().app_data_dir()`.
- **Error type** — all commands return `Result<T, String>`. Errors are human-readable strings.
- **Async commands** — `generate_feature_request` and `create_github_issues` are `async`. The sync commands (credential, repo) are not.
- **IssuePayload** is the shared contract between generator.rs (producer), the frontend editor, and github.rs (consumer). Field changes must be coordinated across all three.
- **No CLI / no tests yet** — there are no integration tests. After implementing, verify with `cargo build` and `cargo clippy -- -D warnings`.

## Common patterns

### Adding a new Tauri command
1. Define `#[tauri::command] pub fn/async fn` in the appropriate module.
2. Add it to `generate_handler![]` in `src/lib.rs`.
3. Document it in the command table in `README.md`.

### rusqlite lifetime gotcha
Never do:
```rust
// WRONG — MappedRows borrows stmt, stmt borrows conn; collect() hits lifetime wall
let rows: Vec<_> = stmt.query_map([], |row| ...)?.collect::<Result<_,_>>()?;
```
Always materialize with an explicit loop:
```rust
// CORRECT
let mapped = stmt.query_map([], |row| { ... })?;
let mut rows = Vec::new();
for r in mapped {
    rows.push(r.map_err(|e| e.to_string())?);
}
```

### Credential access pattern
- Frontend calls: `invoke("plugin:feature-requests|get_credential", { key: "..." })`
- Rust internals (generator, github): `get_credential_internal("anthropic_key")?`
- Never call the Tauri command from Rust — use `get_credential_internal`.

## Build & verify

```bash
cargo build            # must compile clean
cargo clippy -- -D warnings   # must have zero warnings
```

The plugin compiles to a staticlib + cdylib + rlib (all three crate types).

## GitHub label workflow (autonomous implementation loop)

Issues in this repo tagged `ai-implement` are picked up by `.github/workflows/claude-implement.yml`:
1. `ai-implement` applied → Action triggers
2. Action relabels to `ai-in-progress`, then calls Claude Code to implement
3. Claude creates branch `ai/issue-N`, commits, opens PR
4. PR opened → issue labeled `ai-pr-opened`

When implementing an issue, follow these rules:
- Read this file first
- Make the minimal change needed — don't refactor unrelated code
- `cargo build` must succeed before committing
- `cargo clippy -- -D warnings` must pass before committing
- Branch naming: `ai/issue-{number}`
- Commit message format: `feat/fix/chore: brief description (#N)`
