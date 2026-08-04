use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::storage::{Database, dto::ProfileSummary};

const MAX_MANUAL_CONTEXT_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub enum ProfileStoreError {
    Database(rusqlite::Error),
    Invalid(&'static str),
    NotFound,
    RevisionConflict { current: i64 },
    Archived,
    InUse,
}

impl From<rusqlite::Error> for ProfileStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedFact {
    pub id: String,
    pub category: String,
    pub text: String,
    pub source_range: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProvenance {
    pub fixture_id: String,
    pub extraction_model_id: String,
    pub extracted_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VacancyInput {
    pub source_kind: String,
    pub source_value: String,
    pub role_title: String,
    pub company_context: String,
    pub responsibilities: Vec<String>,
    pub requirements: Vec<String>,
    pub review_status: String,
    pub provenance: SourceProvenance,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfigurationInput {
    pub response_model_id: String,
    pub transcription_model_id: String,
    pub translation_language: String,
    pub answer_depth: String,
    pub question_confidence_threshold: f64,
    pub processing_boundary_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProfileInput {
    pub id: Option<String>,
    pub expected_revision: Option<i64>,
    pub name: String,
    pub manual_context: String,
    pub vacancy: Option<VacancyInput>,
    pub model_configuration: Option<ModelConfigurationInput>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VacancySource {
    pub id: String,
    pub source_kind: String,
    pub source_value: String,
    pub role_title: String,
    pub company_context: String,
    pub responsibilities: Vec<String>,
    pub requirements: Vec<String>,
    pub review_status: String,
    pub provenance: SourceProvenance,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSource {
    pub id: String,
    pub kind: String,
    pub display_name: String,
    pub mime_type: Option<String>,
    pub extracted_facts: Vec<ExtractedFact>,
    pub content_status: String,
    pub redaction_summary: Option<String>,
    pub checksum: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfiguration {
    pub id: String,
    pub response_model_id: String,
    pub transcription_model_id: String,
    pub translation_language: String,
    pub answer_depth: String,
    pub question_confidence_threshold: f64,
    pub processing_boundary_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub status: String,
    pub manual_context: String,
    pub model_configuration: Option<ModelConfiguration>,
    pub vacancy: Option<VacancySource>,
    pub sources: Vec<ProfileSource>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub revision: i64,
}

#[derive(Clone, Debug)]
pub struct NewProfileSource<'a> {
    pub id: &'a str,
    pub kind: &'a str,
    pub display_name: &'a str,
    pub mime_type: &'a str,
    pub storage_key: &'a str,
    pub content_status: &'a str,
    pub redaction_summary: Option<&'a str>,
    pub checksum: &'a str,
    pub extracted_facts: &'a [ExtractedFact],
}

pub fn list(
    database: &Database,
    owner_user_id: &str,
) -> Result<Vec<ProfileSummary>, ProfileStoreError> {
    let mut statement = database.connection().prepare(
        "SELECT id, name, status, updated_at_ms, revision FROM ai_profiles \
         WHERE owner_user_id = ?1 ORDER BY updated_at_ms DESC, id DESC",
    )?;
    Ok(statement
        .query_map([owner_user_id], ProfileSummary::from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn get(
    database: &Database,
    owner_user_id: &str,
    profile_id: &str,
) -> Result<Profile, ProfileStoreError> {
    let profile = database
        .connection()
        .query_row(
            "SELECT id, name, status, manual_context, model_configuration_id, \
                    created_at_ms, updated_at_ms, revision \
             FROM ai_profiles WHERE id = ?1 AND owner_user_id = ?2",
            params![profile_id, owner_user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )
        .optional()?
        .ok_or(ProfileStoreError::NotFound)?;

    let model_configuration = profile
        .4
        .as_deref()
        .map(|id| load_model_configuration(database, id))
        .transpose()?;
    let vacancy = load_vacancy(database, profile_id)?;
    let sources = load_sources(database, profile_id)?;
    Ok(Profile {
        id: profile.0,
        name: profile.1,
        status: profile.2,
        manual_context: profile.3,
        model_configuration,
        vacancy,
        sources,
        created_at_ms: profile.5,
        updated_at_ms: profile.6,
        revision: profile.7,
    })
}

pub fn save(
    database: &mut Database,
    owner_user_id: &str,
    input: SaveProfileInput,
    now_ms: i64,
) -> Result<Profile, ProfileStoreError> {
    validate_save(&input)?;
    let profile_id = input.id.clone().unwrap_or_else(|| {
        stable_id(
            "profile",
            &[
                owner_user_id.as_bytes(),
                input.name.as_bytes(),
                &now_ms.to_be_bytes(),
            ],
        )
    });

    database
        .transaction(|transaction| {
            let model_id = input
                .model_configuration
                .as_ref()
                .map(|configuration| save_model_configuration(transaction, configuration))
                .transpose()?;

            if input.id.is_some() {
                update_profile(
                    transaction,
                    owner_user_id,
                    &profile_id,
                    &input,
                    model_id.as_deref(),
                    now_ms,
                )?;
            } else {
                if input.expected_revision.is_some() {
                    return Err(to_sql_error(ProfileStoreError::Invalid(
                        "new profiles cannot have an expected revision",
                    )));
                }
                transaction.execute(
                    "INSERT INTO ai_profiles(\
                    id, owner_user_id, name, status, manual_context, model_configuration_id, \
                    created_at_ms, updated_at_ms, revision\
                 ) VALUES (?1, ?2, ?3, 'draft', ?4, ?5, ?6, ?6, 1)",
                    params![
                        profile_id,
                        owner_user_id,
                        input.name,
                        input.manual_context,
                        model_id,
                        now_ms
                    ],
                )?;
            }

            save_vacancy(transaction, &profile_id, input.vacancy.as_ref())?;
            refresh_readiness(transaction, &profile_id)?;
            Ok(())
        })
        .map_err(from_transaction_error)?;

    get(database, owner_user_id, &profile_id)
}

pub fn insert_source(
    database: &mut Database,
    owner_user_id: &str,
    profile_id: &str,
    expected_revision: i64,
    source: NewProfileSource<'_>,
    now_ms: i64,
) -> Result<Profile, ProfileStoreError> {
    validate_source(&source)?;
    let extracted_facts_json = serde_json::to_string(source.extracted_facts)
        .map_err(|_| ProfileStoreError::Invalid("invalid extracted facts"))?;
    database
        .transaction(|transaction| {
            assert_revision(transaction, owner_user_id, profile_id, expected_revision)?;
            transaction.execute(
                "INSERT INTO profile_sources(\
                    id, profile_id, kind, display_name, mime_type, storage_key, extracted_facts_json, \
                    content_status, redaction_summary, checksum\
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    source.id,
                    profile_id,
                    source.kind,
                    source.display_name,
                    source.mime_type,
                    source.storage_key,
                    extracted_facts_json,
                    source.content_status,
                    source.redaction_summary,
                    source.checksum
                ],
            )?;
            transaction.execute(
                "UPDATE ai_profiles SET updated_at_ms = ?1, revision = revision + 1 WHERE id = ?2",
                params![now_ms, profile_id],
            )?;
            refresh_readiness(transaction, profile_id)?;
            Ok(())
        })
        .map_err(from_transaction_error)?;
    get(database, owner_user_id, profile_id)
}

pub fn archive(
    database: &mut Database,
    owner_user_id: &str,
    profile_id: &str,
    expected_revision: i64,
    now_ms: i64,
) -> Result<Profile, ProfileStoreError> {
    database
        .transaction(|transaction| {
            assert_revision(transaction, owner_user_id, profile_id, expected_revision)?;
            let in_use: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM meetings WHERE profile_id = ?1 \
                 AND status IN ('prepared', 'gating', 'running', 'stopping'))",
                [profile_id],
                |row| row.get(0),
            )?;
            if in_use {
                return Err(to_sql_error(ProfileStoreError::InUse));
            }
            transaction.execute(
                "UPDATE ai_profiles SET status = 'archived', updated_at_ms = ?1, revision = revision + 1 \
                 WHERE id = ?2",
                params![now_ms, profile_id],
            )?;
            Ok(())
        })
        .map_err(from_transaction_error)?;
    get(database, owner_user_id, profile_id)
}

pub fn restore(
    database: &mut Database,
    owner_user_id: &str,
    profile_id: &str,
    expected_revision: i64,
    now_ms: i64,
) -> Result<Profile, ProfileStoreError> {
    database
        .transaction(|transaction| {
            let (_, status) =
                assert_revision(transaction, owner_user_id, profile_id, expected_revision)?;
            if status != "archived" {
                return Err(to_sql_error(ProfileStoreError::Invalid(
                    "profile is not archived",
                )));
            }
            transaction.execute(
                "UPDATE ai_profiles SET status = 'draft', updated_at_ms = ?1, revision = revision + 1 \
                 WHERE id = ?2",
                params![now_ms, profile_id],
            )?;
            refresh_readiness(transaction, profile_id)?;
            Ok(())
        })
        .map_err(from_transaction_error)?;
    get(database, owner_user_id, profile_id)
}

fn validate_save(input: &SaveProfileInput) -> Result<(), ProfileStoreError> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(ProfileStoreError::Invalid(
            "profile name must be 1-100 characters",
        ));
    }
    if input.manual_context.len() > MAX_MANUAL_CONTEXT_BYTES {
        return Err(ProfileStoreError::Invalid("manual context is too large"));
    }
    if let Some(vacancy) = &input.vacancy {
        if !matches!(vacancy.source_kind.as_str(), "url" | "pasted_text")
            || !matches!(
                vacancy.review_status.as_str(),
                "pending" | "needs_review" | "confirmed" | "rejected"
            )
            || vacancy.source_value.trim().is_empty()
        {
            return Err(ProfileStoreError::Invalid("invalid vacancy"));
        }
    }
    if let Some(configuration) = &input.model_configuration {
        if configuration.response_model_id.is_empty()
            || configuration.transcription_model_id.is_empty()
            || configuration.processing_boundary_id.is_empty()
            || !matches!(
                configuration.answer_depth.as_str(),
                "brief" | "balanced" | "detailed"
            )
            || !(0.0..=1.0).contains(&configuration.question_confidence_threshold)
        {
            return Err(ProfileStoreError::Invalid("invalid model configuration"));
        }
    }
    Ok(())
}

fn validate_source(source: &NewProfileSource<'_>) -> Result<(), ProfileStoreError> {
    if !matches!(source.kind, "resume" | "manual" | "project")
        || !matches!(source.content_status, "allowed" | "redacted" | "rejected")
        || source.display_name.trim().is_empty()
        || source.storage_key.is_empty()
        || source.checksum.len() != 64
    {
        return Err(ProfileStoreError::Invalid("invalid profile source"));
    }
    Ok(())
}

fn update_profile(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
    profile_id: &str,
    input: &SaveProfileInput,
    model_configuration_id: Option<&str>,
    now_ms: i64,
) -> rusqlite::Result<()> {
    let expected_revision = input
        .expected_revision
        .ok_or_else(|| to_sql_error(ProfileStoreError::Invalid("expected revision is required")))?;
    let (_, status) = assert_revision(transaction, owner_user_id, profile_id, expected_revision)?;
    if status == "archived" {
        return Err(to_sql_error(ProfileStoreError::Archived));
    }
    transaction.execute(
        "UPDATE ai_profiles SET name = ?1, manual_context = ?2, model_configuration_id = ?3, \
         updated_at_ms = ?4, revision = revision + 1 WHERE id = ?5",
        params![
            input.name.trim(),
            input.manual_context,
            model_configuration_id,
            now_ms,
            profile_id
        ],
    )?;
    Ok(())
}

fn assert_revision(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
    profile_id: &str,
    expected_revision: i64,
) -> rusqlite::Result<(i64, String)> {
    let current = transaction
        .query_row(
            "SELECT revision, status FROM ai_profiles WHERE id = ?1 AND owner_user_id = ?2",
            params![profile_id, owner_user_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| to_sql_error(ProfileStoreError::NotFound))?;
    if current.0 != expected_revision {
        return Err(to_sql_error(ProfileStoreError::RevisionConflict {
            current: current.0,
        }));
    }
    Ok(current)
}

fn save_model_configuration(
    transaction: &Transaction<'_>,
    input: &ModelConfigurationInput,
) -> rusqlite::Result<String> {
    let threshold = input.question_confidence_threshold.to_bits().to_be_bytes();
    let id = stable_id(
        "model",
        &[
            input.response_model_id.as_bytes(),
            input.transcription_model_id.as_bytes(),
            input.translation_language.as_bytes(),
            input.answer_depth.as_bytes(),
            &threshold,
            input.processing_boundary_id.as_bytes(),
        ],
    );
    transaction.execute(
        "INSERT OR IGNORE INTO model_configurations(\
            id, response_model_id, transcription_model_id, translation_language, answer_depth, \
            question_confidence_threshold, processing_boundary_id\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            input.response_model_id,
            input.transcription_model_id,
            input.translation_language,
            input.answer_depth,
            input.question_confidence_threshold,
            input.processing_boundary_id
        ],
    )?;
    Ok(id)
}

fn save_vacancy(
    transaction: &Transaction<'_>,
    profile_id: &str,
    vacancy: Option<&VacancyInput>,
) -> rusqlite::Result<()> {
    let Some(vacancy) = vacancy else {
        transaction.execute(
            "DELETE FROM vacancy_sources WHERE profile_id = ?1",
            [profile_id],
        )?;
        return Ok(());
    };
    let id = stable_id("vacancy", &[profile_id.as_bytes()]);
    transaction.execute(
        "INSERT INTO vacancy_sources(\
            id, profile_id, source_kind, source_value, role_title, company_context, \
            responsibilities_json, requirements_json, review_status, provenance_json\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
         ON CONFLICT(profile_id) DO UPDATE SET \
            source_kind = excluded.source_kind, source_value = excluded.source_value, \
            role_title = excluded.role_title, company_context = excluded.company_context, \
            responsibilities_json = excluded.responsibilities_json, \
            requirements_json = excluded.requirements_json, review_status = excluded.review_status, \
            provenance_json = excluded.provenance_json",
        params![
            id,
            profile_id,
            vacancy.source_kind,
            vacancy.source_value,
            vacancy.role_title,
            vacancy.company_context,
            serde_json::to_string(&vacancy.responsibilities).expect("serializable strings"),
            serde_json::to_string(&vacancy.requirements).expect("serializable strings"),
            vacancy.review_status,
            serde_json::to_string(&vacancy.provenance).expect("serializable provenance")
        ],
    )?;
    Ok(())
}

fn refresh_readiness(transaction: &Transaction<'_>, profile_id: &str) -> rusqlite::Result<()> {
    transaction.execute(
        "UPDATE ai_profiles SET status = CASE WHEN \
            model_configuration_id IS NOT NULL \
            AND NOT EXISTS(SELECT 1 FROM vacancy_sources WHERE profile_id = ?1 AND review_status != 'confirmed') \
            AND NOT EXISTS(SELECT 1 FROM profile_sources WHERE profile_id = ?1 \
                           AND content_status NOT IN ('allowed', 'redacted')) \
            THEN 'ready' ELSE 'draft' END \
         WHERE id = ?1 AND status != 'archived'",
        [profile_id],
    )?;
    Ok(())
}

fn load_model_configuration(
    database: &Database,
    id: &str,
) -> Result<ModelConfiguration, ProfileStoreError> {
    Ok(database.connection().query_row(
        "SELECT id, response_model_id, transcription_model_id, translation_language, answer_depth, \
                question_confidence_threshold, processing_boundary_id \
         FROM model_configurations WHERE id = ?1",
        [id],
        |row| {
            Ok(ModelConfiguration {
                id: row.get(0)?,
                response_model_id: row.get(1)?,
                transcription_model_id: row.get(2)?,
                translation_language: row.get(3)?,
                answer_depth: row.get(4)?,
                question_confidence_threshold: row.get(5)?,
                processing_boundary_id: row.get(6)?,
            })
        },
    )?)
}

fn load_vacancy(
    database: &Database,
    profile_id: &str,
) -> Result<Option<VacancySource>, ProfileStoreError> {
    Ok(database
        .connection()
        .query_row(
            "SELECT id, source_kind, source_value, role_title, company_context, \
                    responsibilities_json, requirements_json, review_status, provenance_json \
             FROM vacancy_sources WHERE profile_id = ?1",
            [profile_id],
            |row| {
                let responsibilities: String = row.get(5)?;
                let requirements: String = row.get(6)?;
                let provenance: String = row.get(8)?;
                Ok(VacancySource {
                    id: row.get(0)?,
                    source_kind: row.get(1)?,
                    source_value: row.get(2)?,
                    role_title: row.get(3)?,
                    company_context: row.get(4)?,
                    responsibilities: serde_json::from_str(&responsibilities)
                        .map_err(json_error)?,
                    requirements: serde_json::from_str(&requirements).map_err(json_error)?,
                    review_status: row.get(7)?,
                    provenance: serde_json::from_str(&provenance).map_err(json_error)?,
                })
            },
        )
        .optional()?)
}

fn load_sources(
    database: &Database,
    profile_id: &str,
) -> Result<Vec<ProfileSource>, ProfileStoreError> {
    let mut statement = database.connection().prepare(
        "SELECT id, kind, display_name, mime_type, extracted_facts_json, content_status, \
                redaction_summary, checksum FROM profile_sources WHERE profile_id = ?1 ORDER BY id",
    )?;
    Ok(statement
        .query_map([profile_id], |row| {
            let facts: String = row.get(4)?;
            Ok(ProfileSource {
                id: row.get(0)?,
                kind: row.get(1)?,
                display_name: row.get(2)?,
                mime_type: row.get(3)?,
                extracted_facts: serde_json::from_str(&facts).map_err(json_error)?,
                content_status: row.get(5)?,
                redaction_summary: row.get(6)?,
                checksum: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn stable_id(prefix: &str, parts: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    format!("{prefix}-{}", &format!("{:x}", digest.finalize())[..24])
}

fn json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn to_sql_error(error: ProfileStoreError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(ProfileTransactionError(error)))
}

fn from_transaction_error(error: rusqlite::Error) -> ProfileStoreError {
    match error {
        rusqlite::Error::ToSqlConversionFailure(error) => error
            .downcast::<ProfileTransactionError>()
            .map(|error| error.0)
            .unwrap_or_else(|error| {
                ProfileStoreError::Database(rusqlite::Error::ToSqlConversionFailure(error))
            }),
        error => ProfileStoreError::Database(error),
    }
}

#[derive(Debug)]
struct ProfileTransactionError(ProfileStoreError);

impl std::fmt::Display for ProfileTransactionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("profile transaction failed")
    }
}

impl std::error::Error for ProfileTransactionError {}
