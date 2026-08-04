use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    commands::authorize_window,
    error::CommandError,
    state::AppState,
    storage::{
        AppDataFiles, Database, StorageKey,
        profiles::{self, ExtractedFact, NewProfileSource, Profile, SaveProfileInput},
    },
};

const MAX_FIXTURE_BYTES: u64 = 2 * 1024 * 1024;
static SOURCE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct ProfileCommandState {
    pub database: Mutex<Database>,
    pub files: AppDataFiles,
    pub fixture_root: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveProfileInput {
    pub profile_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreProfileInput {
    pub profile_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfileSourceInput {
    pub profile_id: String,
    pub expected_revision: i64,
    pub kind: String,
    pub fixture_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileListItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub updated_at_ms: i64,
    pub revision: i64,
}

#[tauri::command]
pub fn profile_list(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
) -> Result<Vec<ProfileListItem>, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    profiles::list(&database, &owner)
        .map(|items| {
            items
                .into_iter()
                .map(|item| ProfileListItem {
                    id: item.id,
                    name: item.name,
                    status: item.status,
                    updated_at_ms: item.updated_at_ms,
                    revision: item.revision,
                })
                .collect()
        })
        .map_err(command_error)
}

#[tauri::command]
pub fn profile_get(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    profile_id: String,
) -> Result<Profile, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let database = profile_state.database.lock().map_err(|_| state_error())?;
    profiles::get(&database, &owner, &profile_id).map_err(command_error)
}

#[tauri::command]
pub fn profile_save(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: SaveProfileInput,
) -> Result<Profile, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    profiles::save(&mut database, &owner, input, now_ms()).map_err(command_error)
}

#[tauri::command]
pub fn profile_archive(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: ArchiveProfileInput,
) -> Result<Profile, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    profiles::archive(
        &mut database,
        &owner,
        &input.profile_id,
        input.expected_revision,
        now_ms(),
    )
    .map_err(command_error)
}

#[tauri::command]
pub fn profile_restore(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: RestoreProfileInput,
) -> Result<Profile, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    profiles::restore(
        &mut database,
        &owner,
        &input.profile_id,
        input.expected_revision,
        now_ms(),
    )
    .map_err(command_error)
}

#[tauri::command]
pub fn profile_source_import(
    window: tauri::Window,
    app_state: tauri::State<'_, AppState>,
    profile_state: tauri::State<'_, ProfileCommandState>,
    input: ImportProfileSourceInput,
) -> Result<Profile, CommandError> {
    authorize_window(window.label(), &["main"])?;
    let owner = active_user(&app_state)?;
    let fixture_root = std::fs::canonicalize(&profile_state.fixture_root).map_err(|_| {
        CommandError::new(
            "PROFILE_SOURCE_UNAVAILABLE",
            "The approved fixture source is unavailable",
        )
    })?;
    let fixture_name = fixture_file(&input.fixture_id, &input.kind)?;
    let source_path = std::fs::canonicalize(fixture_root.join(fixture_name)).map_err(|_| {
        CommandError::new(
            "PROFILE_SOURCE_UNAVAILABLE",
            "The approved fixture source is unavailable",
        )
    })?;
    if !source_path.starts_with(&fixture_root) {
        return Err(CommandError::new(
            "PROFILE_SOURCE_NOT_ALLOWED",
            "Only approved fixture files can be imported",
        ));
    }
    let metadata = std::fs::metadata(&source_path).map_err(|_| {
        CommandError::new(
            "PROFILE_SOURCE_UNAVAILABLE",
            "The approved fixture source is unavailable",
        )
    })?;
    if !metadata.is_file() || metadata.len() > MAX_FIXTURE_BYTES {
        return Err(CommandError::new(
            "PROFILE_SOURCE_NOT_ALLOWED",
            "The fixture file is not supported",
        ));
    }
    let (mime_type, extension) = approved_type(&source_path)?;
    let bytes = std::fs::read(&source_path).map_err(|_| {
        CommandError::new(
            "PROFILE_SOURCE_UNAVAILABLE",
            "The approved fixture source is unavailable",
        )
    })?;
    let scan = scan_fixture(&bytes)?;
    let checksum = format!("{:x}", Sha256::digest(&scan.bytes));
    let nonce = SOURCE_SEQUENCE
        .fetch_add(1, Ordering::Relaxed)
        .to_be_bytes();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_be_bytes();
    let source_id = profiles::stable_id(
        "source",
        &[
            input.profile_id.as_bytes(),
            input.kind.as_bytes(),
            checksum.as_bytes(),
            &timestamp,
            &nonce,
        ],
    );
    let storage_key =
        StorageKey::parse(format!("profiles/{source_id}.{extension}")).map_err(|_| {
            CommandError::new(
                "PROFILE_SOURCE_NOT_ALLOWED",
                "The fixture file is not supported",
            )
        })?;
    profile_state
        .files
        .write(&storage_key, &scan.bytes)
        .map_err(|_| {
            CommandError::new(
                "PROFILE_SOURCE_STORE_FAILED",
                "The approved fixture could not be stored",
            )
        })?;

    let display_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("fixture");
    let extracted_facts = fixture_facts(&input.fixture_id);
    let mut database = profile_state.database.lock().map_err(|_| state_error())?;
    let result = profiles::insert_source(
        &mut database,
        &owner,
        &input.profile_id,
        input.expected_revision,
        NewProfileSource {
            id: &source_id,
            kind: &input.kind,
            display_name,
            mime_type,
            storage_key: storage_key.as_str(),
            content_status: scan.status,
            redaction_summary: scan.summary,
            checksum: &checksum,
            extracted_facts: &extracted_facts,
        },
        now_ms(),
    );
    if result.is_err() {
        let _ = profile_state.files.delete(&storage_key);
    }
    result.map_err(command_error)
}

fn fixture_file(fixture_id: &str, kind: &str) -> Result<&'static str, CommandError> {
    match (fixture_id, kind) {
        ("resume-product-engineer", "resume") => Ok("resume-product-engineer.md"),
        ("project-performance", "project") => Ok("project-performance.md"),
        _ => Err(CommandError::new(
            "PROFILE_SOURCE_NOT_ALLOWED",
            "Only approved profile fixtures can be imported",
        )),
    }
}

fn fixture_facts(fixture_id: &str) -> Vec<ExtractedFact> {
    match fixture_id {
        "resume-product-engineer" => vec![ExtractedFact {
            id: "fact-resume-role".to_owned(),
            category: "experience".to_owned(),
            text: "Led frontend delivery for a product team".to_owned(),
            source_range: "section:experience".to_owned(),
        }],
        "project-performance" => vec![ExtractedFact {
            id: "fact-project-result".to_owned(),
            category: "project_result".to_owned(),
            text: "Reduced page load time by 30%".to_owned(),
            source_range: "section:outcome".to_owned(),
        }],
        _ => Vec::new(),
    }
}

fn active_user(state: &AppState) -> Result<String, CommandError> {
    state
        .0
        .lock()
        .map_err(|_| state_error())?
        .active_user_id
        .clone()
        .ok_or_else(|| CommandError::new("AUTH_REQUIRED", "Sign in to manage interview profiles"))
}

fn approved_type(path: &std::path::Path) -> Result<(&'static str, &'static str), CommandError> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("txt") => Ok(("text/plain", "txt")),
        Some("md") => Ok(("text/markdown", "md")),
        Some("pdf") => Ok(("application/pdf", "pdf")),
        _ => Err(CommandError::new(
            "PROFILE_SOURCE_NOT_ALLOWED",
            "Use an approved PDF, Markdown, or text fixture",
        )),
    }
}

struct ScanResult {
    bytes: Vec<u8>,
    status: &'static str,
    summary: Option<&'static str>,
}

fn scan_fixture(bytes: &[u8]) -> Result<ScanResult, CommandError> {
    if bytes.starts_with(b"%PDF-") {
        if [
            b"SECRET=".as_slice(),
            b"TOKEN=".as_slice(),
            b"PASSWORD=".as_slice(),
        ]
        .iter()
        .any(|marker| bytes.windows(marker.len()).any(|window| window == *marker))
        {
            return Err(CommandError::new(
                "PROFILE_SOURCE_REJECTED",
                "The PDF fixture contains sensitive markers and cannot be safely redacted",
            ));
        }
        return Ok(ScanResult {
            bytes: bytes.to_vec(),
            status: "allowed",
            summary: None,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| {
        CommandError::new(
            "PROFILE_SOURCE_REJECTED",
            "The fixture content could not be safely scanned",
        )
    })?;
    let mut redacted = text.to_owned();
    let mut changed = false;
    for marker in ["SECRET=", "TOKEN=", "PASSWORD="] {
        for line in redacted
            .lines()
            .filter(|line| line.contains(marker))
            .map(str::to_owned)
            .collect::<Vec<_>>()
        {
            redacted = redacted.replace(&line, "[REDACTED]");
            changed = true;
        }
    }
    Ok(ScanResult {
        bytes: redacted.into_bytes(),
        status: if changed { "redacted" } else { "allowed" },
        summary: changed.then_some("Sensitive fixture lines were removed before storage"),
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn state_error() -> CommandError {
    CommandError::new(
        "STATE_UNAVAILABLE",
        "Profile state is temporarily unavailable",
    )
    .retryable(None)
}

fn command_error(error: profiles::ProfileStoreError) -> CommandError {
    match error {
        profiles::ProfileStoreError::Invalid(message) => {
            CommandError::new("PROFILE_INVALID", message)
        }
        profiles::ProfileStoreError::NotFound => {
            CommandError::new("PROFILE_NOT_FOUND", "Profile not found")
        }
        profiles::ProfileStoreError::RevisionConflict { current } => CommandError::new(
            "PROFILE_REVISION_CONFLICT",
            format!("Profile changed since it was opened (current revision {current})"),
        ),
        profiles::ProfileStoreError::Archived => {
            CommandError::new("PROFILE_ARCHIVED", "Archived profiles cannot be edited")
        }
        profiles::ProfileStoreError::InUse => CommandError::new(
            "PROFILE_IN_USE",
            "Stop the active meeting before archiving this profile",
        ),
        profiles::ProfileStoreError::Database(_) => CommandError::new(
            "PROFILE_STORAGE_FAILED",
            "The profile operation could not be completed",
        )
        .retryable(None),
    }
}
