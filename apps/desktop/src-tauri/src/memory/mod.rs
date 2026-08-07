use std::{collections::HashSet, env, future::Future, pin::Pin};

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{ProfileDocument, UserCorrection},
};

const DEFAULT_BASE_URL: &str = "https://api.evermind.ai";
const DEFAULT_APP_ID: &str = "knowu";
const DEFAULT_PROJECT_ID: &str = "knowu-hackathon";
const APPROVED_MEMORY_SESSION: &str = "knowu-approved-memories";

type MemoryFuture<'a, T> = Pin<Box<dyn Future<Output = AppResult<T>> + Send + 'a>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecord {
    pub id: String,
    pub text: String,
    pub memory_type: String,
    pub source: String,
    pub created_at: i64,
    pub importance: Option<f64>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWriteReceipt {
    pub provider: String,
    pub stored_count: usize,
    pub message: String,
}

#[allow(dead_code)]
pub trait MemoryService {
    fn is_configured(&self) -> bool;
    fn store_memory<'a>(&'a self, memory: &'a MemoryRecord)
        -> MemoryFuture<'a, MemoryWriteReceipt>;
    fn store_memories<'a>(
        &'a self,
        memories: &'a [MemoryRecord],
    ) -> MemoryFuture<'a, MemoryWriteReceipt>;
    fn search_memories<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> MemoryFuture<'a, Vec<MemoryRecord>>;
    fn get_relevant_context<'a>(&'a self, query: &'a str, limit: usize)
        -> MemoryFuture<'a, String>;
}

#[derive(Clone)]
pub struct EverOSMemoryService {
    http: Client,
    api_key: Option<String>,
    base_url: String,
    user_id: String,
    app_id: String,
    project_id: String,
}

impl Default for EverOSMemoryService {
    fn default() -> Self {
        Self::from_env()
    }
}

impl EverOSMemoryService {
    pub fn from_env() -> Self {
        Self {
            http: Client::builder()
                .user_agent("KnowU/0.2")
                .build()
                .expect("EverOS HTTP client"),
            api_key: non_empty_env("EVEROS_API_KEY"),
            base_url: non_empty_env("EVEROS_BASE_URL")
                .unwrap_or_else(|| DEFAULT_BASE_URL.into())
                .trim_end_matches('/')
                .into(),
            user_id: non_empty_env("EVEROS_USER_ID").unwrap_or_else(|| "knowu-local-user".into()),
            app_id: non_empty_env("EVEROS_APP_ID").unwrap_or_else(|| DEFAULT_APP_ID.into()),
            project_id: non_empty_env("EVEROS_PROJECT_ID")
                .unwrap_or_else(|| DEFAULT_PROJECT_ID.into()),
        }
    }

    pub fn configuration_message(&self) -> String {
        if self.is_configured() {
            "EverOS v2 is configured for approved persistent memories.".into()
        } else {
            "Set EVEROS_API_KEY to enable persistent EverOS memory. KnowU will use safe local profile memories until then.".into()
        }
    }

    async fn post(&self, path: &str, body: Value) -> AppResult<Value> {
        let key = self.api_key.as_deref().ok_or_else(|| {
            AppError::InvalidInput(
                "EverOS is not configured. Set EVEROS_API_KEY and restart KnowU.".into(),
            )
        })?;
        let response = self
            .http
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(key)
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or_else(|_| json!({}));
        if status.is_success() {
            Ok(value)
        } else {
            Err(everos_status_error(status, &value))
        }
    }

    async fn add_and_flush(&self, memories: &[MemoryRecord]) -> AppResult<MemoryWriteReceipt> {
        if memories.is_empty() {
            return Ok(MemoryWriteReceipt {
                provider: "everos".into(),
                stored_count: 0,
                message: "No approved memories needed syncing.".into(),
            });
        }
        self.post(
            "/api/v2/memory/add",
            add_request(memories, &self.user_id, &self.app_id, &self.project_id),
        )
        .await?;
        self.post(
            "/api/v2/memory/flush",
            json!({
                "session_id":APPROVED_MEMORY_SESSION,
                "app_id":self.app_id,
                "project_id":self.project_id
            }),
        )
        .await?;
        Ok(MemoryWriteReceipt {
            provider: "everos".into(),
            stored_count: memories.len(),
            message: format!("Synced {} approved memories to EverOS.", memories.len()),
        })
    }

    async fn search(&self, query: &str, limit: usize) -> AppResult<Vec<MemoryRecord>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        let response = self
            .post(
                "/api/v2/memory/search",
                json!({
                    "query":query.trim(),
                    "app_id":self.app_id,
                    "project_id":self.project_id,
                    "user_id":self.user_id,
                    "method":"hybrid",
                    "top_k":limit.max(1),
                    "include_profile":true,
                    "filters":{"session_id":APPROVED_MEMORY_SESSION}
                }),
            )
            .await?;
        Ok(parse_search_response(&response, limit))
    }
}

impl MemoryService for EverOSMemoryService {
    fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    fn store_memory<'a>(
        &'a self,
        memory: &'a MemoryRecord,
    ) -> MemoryFuture<'a, MemoryWriteReceipt> {
        Box::pin(async move { self.add_and_flush(std::slice::from_ref(memory)).await })
    }

    fn store_memories<'a>(
        &'a self,
        memories: &'a [MemoryRecord],
    ) -> MemoryFuture<'a, MemoryWriteReceipt> {
        Box::pin(async move { self.add_and_flush(memories).await })
    }

    fn search_memories<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> MemoryFuture<'a, Vec<MemoryRecord>> {
        Box::pin(async move { self.search(query, limit).await })
    }

    fn get_relevant_context<'a>(
        &'a self,
        query: &'a str,
        limit: usize,
    ) -> MemoryFuture<'a, String> {
        Box::pin(async move {
            let memories = self.search(query, limit).await?;
            Ok(memories
                .iter()
                .map(|memory| format!("- {}", memory.text))
                .collect::<Vec<_>>()
                .join("\n"))
        })
    }
}

pub fn approved_profile_memories(
    profile: &ProfileDocument,
    corrections: &[UserCorrection],
) -> Vec<MemoryRecord> {
    let now = Utc::now().timestamp();
    let mut values = Vec::new();
    if !profile.summary.trim().is_empty() {
        values.push(memory("profile", "approved_profile", &profile.summary, now));
    }
    for value in &profile.interests {
        values.push(memory("interest", "approved_profile", value, now));
    }
    for value in &profile.active_projects {
        values.push(memory("project", "approved_profile", value, now));
    }
    for value in &profile.patterns {
        values.push(memory("pattern", "approved_profile", value, now));
    }
    for correction in corrections {
        let text = if correction.value.trim().is_empty() {
            correction.subject.clone()
        } else {
            format!("{} — {}", correction.subject, correction.value)
        };
        values.push(memory(
            "preference",
            "explicit_user",
            &text,
            correction.updated_at,
        ));
    }
    values
}

pub fn correction_memory(correction: &UserCorrection) -> MemoryRecord {
    let text = if correction.value.trim().is_empty() {
        correction.subject.clone()
    } else {
        format!("{} — {}", correction.subject, correction.value)
    };
    memory("preference", "explicit_user", &text, correction.updated_at)
}

pub fn demo_seed_memories() -> Vec<MemoryRecord> {
    let now = Utc::now().timestamp();
    [
        (
            "project",
            "Building KnowU, an AI memory product for the EverMind and Snowflake hackathon.",
        ),
        (
            "preference",
            "Prefers local-first architecture and keeping raw personal activity on-device.",
        ),
        (
            "priority",
            "Privacy is more important than adding more features.",
        ),
    ]
    .into_iter()
    .map(|(memory_type, text)| memory(memory_type, "demo_seed", text, now))
    .collect()
}

pub fn safe_local_search(
    profile: &ProfileDocument,
    corrections: &[UserCorrection],
    query: &str,
    limit: usize,
) -> Vec<MemoryRecord> {
    let query_terms = terms(query);
    let mut memories = approved_profile_memories(profile, corrections)
        .into_iter()
        .map(|mut memory| {
            let memory_terms = terms(&memory.text);
            let overlap = query_terms.intersection(&memory_terms).count() as f64;
            let authoritative = (memory.source == "explicit_user") as u8 as f64;
            memory.score = Some((overlap * 0.2 + authoritative * 0.3).min(1.0));
            memory
        })
        .collect::<Vec<_>>();
    memories.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    memories.truncate(limit);
    memories
}

fn memory(memory_type: &str, source: &str, text: &str, created_at: i64) -> MemoryRecord {
    MemoryRecord {
        id: Uuid::new_v4().to_string(),
        text: text.trim().into(),
        memory_type: memory_type.into(),
        source: source.into(),
        created_at,
        importance: None,
        score: None,
    }
}

fn encode_memory(memory: &MemoryRecord) -> String {
    format!(
        "KnowU approved memory [type={}; source={}]: {}",
        memory.memory_type, memory.source, memory.text
    )
}

fn add_request(memories: &[MemoryRecord], user_id: &str, app_id: &str, project_id: &str) -> Value {
    let messages = memories
        .iter()
        .map(|memory| {
            json!({
                "sender_id":user_id,
                "role":"user",
                "timestamp":to_everos_timestamp_millis(memory.created_at),
                "content":encode_memory(memory)
            })
        })
        .collect::<Vec<_>>();
    json!({
        "session_id":APPROVED_MEMORY_SESSION,
        "messages":messages,
        "app_id":app_id,
        "project_id":project_id,
        "mode":"chat",
        "async_mode":false
    })
}

fn to_everos_timestamp_millis(timestamp_seconds: i64) -> i64 {
    timestamp_seconds.saturating_mul(1_000)
}

fn decode_memory(value: &str) -> (String, String, String) {
    let prefix = "KnowU approved memory [type=";
    if let Some(rest) = value.trim().strip_prefix(prefix) {
        if let Some((metadata, text)) = rest.split_once("]: ") {
            let (memory_type, source) = metadata
                .split_once("; source=")
                .unwrap_or((metadata, "everos"));
            return (text.trim().into(), memory_type.into(), source.into());
        }
    }
    (value.trim().into(), "memory".into(), "everos".into())
}

fn parse_search_response(value: &Value, limit: usize) -> Vec<MemoryRecord> {
    let mut memories = Vec::new();
    let mut seen = HashSet::new();
    let data = &value["data"];
    if let Some(episodes) = data["episodes"].as_array() {
        for episode in episodes {
            let created_at = parse_everos_timestamp(&episode["timestamp"]);
            let episode_score = episode["score"].as_f64();
            let facts = episode["atomic_facts"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if facts.is_empty() {
                let text = episode["summary"]
                    .as_str()
                    .or_else(|| episode["episode"].as_str())
                    .unwrap_or_default();
                push_parsed(
                    &mut memories,
                    &mut seen,
                    episode["id"].as_str().unwrap_or_default(),
                    text,
                    created_at,
                    episode_score,
                );
            } else {
                for fact in facts {
                    push_parsed(
                        &mut memories,
                        &mut seen,
                        fact["id"].as_str().unwrap_or_default(),
                        fact["content"].as_str().unwrap_or_default(),
                        created_at,
                        fact["score"].as_f64().or(episode_score),
                    );
                }
            }
        }
    }
    if let Some(messages) = data["unprocessed_messages"].as_array() {
        for message in messages {
            let created_at = parse_everos_timestamp(&message["timestamp"]);
            push_parsed(
                &mut memories,
                &mut seen,
                message["id"].as_str().unwrap_or_default(),
                message["content"].as_str().unwrap_or_default(),
                created_at,
                None,
            );
        }
    }
    memories.truncate(limit);
    memories
}

fn parse_everos_timestamp(value: &Value) -> i64 {
    if let Some(timestamp) = value.as_i64() {
        return if timestamp >= 1_000_000_000_000 {
            timestamp / 1_000
        } else {
            timestamp
        };
    }
    value
        .as_str()
        .and_then(|timestamp| DateTime::parse_from_rfc3339(timestamp).ok())
        .map(|timestamp| timestamp.timestamp())
        .unwrap_or_default()
}

fn push_parsed(
    memories: &mut Vec<MemoryRecord>,
    seen: &mut HashSet<String>,
    id: &str,
    raw_text: &str,
    created_at: i64,
    score: Option<f64>,
) {
    let (text, memory_type, source) = decode_memory(raw_text);
    if text.is_empty() || !seen.insert(text.clone()) {
        return;
    }
    memories.push(MemoryRecord {
        id: if id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            id.into()
        },
        text,
        memory_type,
        source,
        created_at,
        importance: None,
        score,
    });
}

fn terms(value: &str) -> HashSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| term.len() > 2)
        .map(str::to_ascii_lowercase)
        .collect()
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn everos_status_error(status: StatusCode, body: &Value) -> AppError {
    let detail = body["error"]["message"]
        .as_str()
        .or_else(|| body["detail"].as_str())
        .or_else(|| body["message"].as_str())
        .unwrap_or_default();
    let message = match status.as_u16() {
        401 => "EverOS rejected the API key.",
        403 if body["error"]["code"].as_str() == Some("VERSION_NOT_ALLOWED") => {
            "This EverOS account is not enabled for the v2 Memory API."
        }
        403 => "EverOS denied the memory request.",
        429 => "EverOS rate limit reached.",
        500..=599 => "EverOS is temporarily unavailable.",
        _ if !detail.is_empty() => detail,
        _ => "EverOS rejected the memory request.",
    };
    AppError::Provider(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_request_uses_documented_v2_contract_and_millisecond_timestamps() {
        let memories = vec![memory(
            "preference",
            "explicit_user",
            "Privacy first.",
            1_754_563_200,
        )];

        let request = add_request(&memories, "user-1", "knowu", "project-1");

        assert_eq!(request["session_id"], APPROVED_MEMORY_SESSION);
        assert_eq!(request["app_id"], "knowu");
        assert_eq!(request["project_id"], "project-1");
        assert_eq!(request["mode"], "chat");
        assert_eq!(request["async_mode"], false);
        assert_eq!(request["messages"][0]["sender_id"], "user-1");
        assert_eq!(request["messages"][0]["role"], "user");
        assert_eq!(request["messages"][0]["timestamp"], 1_754_563_200_000_i64);
    }

    #[test]
    fn parses_atomic_facts_and_preserves_metadata() {
        let result = parse_search_response(
            &json!({"data":{"episodes":[{
                "id":"episode-1",
                "timestamp":"2026-08-07T12:00:00Z",
                "score":0.8,
                "atomic_facts":[{"id":"fact-1","content":"KnowU approved memory [type=preference; source=explicit_user]: Privacy first.","score":0.95}]
            }]}}),
            3,
        );

        assert_eq!(result[0].text, "Privacy first.");
        assert_eq!(result[0].memory_type, "preference");
        assert_eq!(result[0].source, "explicit_user");
        assert_eq!(result[0].score, Some(0.95));
    }

    #[test]
    fn parses_unprocessed_message_millisecond_timestamps() {
        let result = parse_search_response(
            &json!({"data":{"unprocessed_messages":[{
                "id":"message-1",
                "timestamp":1_754_563_200_000_i64,
                "content":"KnowU approved memory [type=project; source=approved_profile]: Ship KnowU."
            }]}}),
            3,
        );

        assert_eq!(result[0].created_at, 1_754_563_200);
        assert_eq!(result[0].text, "Ship KnowU.");
    }

    #[test]
    fn surfaces_nested_v2_validation_errors() {
        let error = everos_status_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({"error":{"code":"invalid_argument","message":"exactly one owner is required"}}),
        );

        assert_eq!(
            error.to_string(),
            "provider rejected the request: exactly one owner is required"
        );
    }

    #[test]
    fn local_fallback_prioritizes_explicit_user_truth() {
        let profile = ProfileDocument {
            summary: "Building a memory app".into(),
            ..Default::default()
        };
        let corrections = vec![UserCorrection {
            id: "truth".into(),
            subject: "Privacy is the priority".into(),
            value: String::new(),
            created_at: 1,
            updated_at: 1,
        }];

        let result = safe_local_search(&profile, &corrections, "What privacy priority matters?", 1);
        assert_eq!(result[0].source, "explicit_user");
    }
}
