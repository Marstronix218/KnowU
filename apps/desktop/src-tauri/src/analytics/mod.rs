use std::env;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMeasurement {
    pub baseline_input_tokens: i64,
    pub optimized_input_tokens: i64,
    pub measurement_method: String,
}

impl TokenMeasurement {
    pub fn tokens_saved(&self) -> i64 {
        (self.baseline_input_tokens - self.optimized_input_tokens).max(0)
    }

    pub fn reduction_percent(&self) -> f64 {
        if self.baseline_input_tokens <= 0 {
            0.0
        } else {
            self.tokens_saved() as f64 * 100.0 / self.baseline_input_tokens as f64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceRun {
    pub id: String,
    pub timestamp: String,
    pub model: String,
    pub baseline_input_tokens: i64,
    pub optimized_input_tokens: i64,
    pub tokens_saved: i64,
    pub reduction_percent: f64,
    pub actual_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub context_budget_tokens: i64,
    pub context_estimated_tokens: i64,
    pub context_units_considered: i64,
    pub context_units_sent: i64,
    pub context_units_omitted: i64,
    pub context_detail_level: String,
    pub provider_preflight_input_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_write_input_tokens: Option<i64>,
    pub latency_ms: i64,
    pub estimated_cost_usd: Option<f64>,
    pub memory_count: i64,
    pub mode: String,
    pub memory_provider: String,
    pub measurement_method: String,
}

#[derive(Clone)]
pub struct SnowflakeAnalyticsService {
    http: Client,
    account_url: Option<String>,
    token: Option<String>,
    token_type: String,
    warehouse: Option<String>,
    database: Option<String>,
    schema: Option<String>,
    role: Option<String>,
    token_model: String,
}

impl Default for SnowflakeAnalyticsService {
    fn default() -> Self {
        Self::from_env()
    }
}

impl SnowflakeAnalyticsService {
    pub fn from_env() -> Self {
        Self {
            http: Client::builder()
                .user_agent("KnowU/0.2")
                .build()
                .expect("Snowflake HTTP client"),
            account_url: non_empty_env("SNOWFLAKE_ACCOUNT_URL")
                .map(|value| value.trim_end_matches('/').into()),
            token: non_empty_env("SNOWFLAKE_PROGRAMMATIC_ACCESS_TOKEN")
                .or_else(|| non_empty_env("SNOWFLAKE_OAUTH_TOKEN")),
            token_type: if non_empty_env("SNOWFLAKE_PROGRAMMATIC_ACCESS_TOKEN").is_some() {
                "PROGRAMMATIC_ACCESS_TOKEN".into()
            } else {
                "OAUTH".into()
            },
            warehouse: non_empty_env("SNOWFLAKE_WAREHOUSE"),
            database: non_empty_env("SNOWFLAKE_DATABASE"),
            schema: non_empty_env("SNOWFLAKE_SCHEMA"),
            role: non_empty_env("SNOWFLAKE_ROLE"),
            token_model: non_empty_env("SNOWFLAKE_TOKEN_MODEL")
                .unwrap_or_else(|| "llama3.3-70b".into()),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.account_url.is_some() && self.token.is_some()
    }

    pub fn prompt_tokenization_enabled(&self) -> bool {
        matches!(
            non_empty_env("SNOWFLAKE_ENABLE_AI_COUNT_TOKENS")
                .as_deref()
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("1" | "true" | "yes")
        )
    }

    pub fn configuration_message(&self) -> String {
        if self.is_configured() {
            if self.prompt_tokenization_enabled() {
                format!(
                    "Snowflake telemetry is configured; approved prompt counts use AI_COUNT_TOKENS with {}.",
                    self.token_model
                )
            } else {
                "Snowflake aggregate telemetry is configured; provider usage anchors the private local token comparison.".into()
            }
        } else {
            "Set SNOWFLAKE_ACCOUNT_URL and a Snowflake PAT or OAuth token to sync telemetry. Local aggregate telemetry remains available meanwhile.".into()
        }
    }

    pub async fn measure_tokens(
        &self,
        baseline_prompt: &str,
        optimized_prompt: &str,
    ) -> AppResult<TokenMeasurement> {
        let response = self
            .execute(
                "SELECT AI_COUNT_TOKENS('ai_complete', ?, ?, TRUE):value::INTEGER, AI_COUNT_TOKENS('ai_complete', ?, ?, TRUE):value::INTEGER",
                vec![
                    binding("TEXT", &self.token_model),
                    binding("TEXT", baseline_prompt),
                    binding("TEXT", &self.token_model),
                    binding("TEXT", optimized_prompt),
                ],
            )
            .await?;
        let row = response["data"]
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::Provider("Snowflake returned no token-count row.".into()))?;
        let baseline = parse_i64(row.first()).ok_or_else(|| {
            AppError::Provider("Snowflake could not count the baseline prompt.".into())
        })?;
        let optimized = parse_i64(row.get(1)).ok_or_else(|| {
            AppError::Provider("Snowflake could not count the KnowU prompt.".into())
        })?;
        Ok(TokenMeasurement {
            baseline_input_tokens: baseline,
            optimized_input_tokens: optimized,
            measurement_method: format!("snowflake_ai_count_tokens:{}", self.token_model),
        })
    }

    pub async fn record_inference_run(&self, run: &InferenceRun) -> AppResult<()> {
        self.execute(
            "INSERT INTO INFERENCE_RUNS (ID, TIMESTAMP, MODEL, BASELINE_INPUT_TOKENS, OPTIMIZED_INPUT_TOKENS, TOKENS_SAVED, REDUCTION_PERCENT, ACTUAL_INPUT_TOKENS, OUTPUT_TOKENS, CONTEXT_BUDGET_TOKENS, CONTEXT_ESTIMATED_TOKENS, CONTEXT_UNITS_CONSIDERED, CONTEXT_UNITS_SENT, CONTEXT_UNITS_OMITTED, CONTEXT_DETAIL_LEVEL, PROVIDER_PREFLIGHT_INPUT_TOKENS, CACHE_READ_INPUT_TOKENS, CACHE_WRITE_INPUT_TOKENS, LATENCY_MS, ESTIMATED_COST_USD, MEMORY_COUNT, MODE, MEMORY_PROVIDER, MEASUREMENT_METHOD) SELECT ?, TO_TIMESTAMP_TZ(?), ?, ?::NUMBER, ?::NUMBER, ?::NUMBER, ?::FLOAT, TRY_TO_NUMBER(?), TRY_TO_NUMBER(?), ?::NUMBER, ?::NUMBER, ?::NUMBER, ?::NUMBER, ?::NUMBER, ?, TRY_TO_NUMBER(?), TRY_TO_NUMBER(?), TRY_TO_NUMBER(?), ?::NUMBER, TRY_TO_DOUBLE(?), ?::NUMBER, ?, ?, ?",
            vec![
                binding("TEXT", &run.id),
                binding("TEXT", &run.timestamp),
                binding("TEXT", &run.model),
                binding("TEXT", &run.baseline_input_tokens.to_string()),
                binding("TEXT", &run.optimized_input_tokens.to_string()),
                binding("TEXT", &run.tokens_saved.to_string()),
                binding("TEXT", &run.reduction_percent.to_string()),
                binding("TEXT", &optional_number(run.actual_input_tokens)),
                binding("TEXT", &optional_number(run.output_tokens)),
                binding("TEXT", &run.context_budget_tokens.to_string()),
                binding("TEXT", &run.context_estimated_tokens.to_string()),
                binding("TEXT", &run.context_units_considered.to_string()),
                binding("TEXT", &run.context_units_sent.to_string()),
                binding("TEXT", &run.context_units_omitted.to_string()),
                binding("TEXT", &run.context_detail_level),
                binding(
                    "TEXT",
                    &optional_number(run.provider_preflight_input_tokens),
                ),
                binding("TEXT", &optional_number(run.cache_read_input_tokens)),
                binding("TEXT", &optional_number(run.cache_write_input_tokens)),
                binding("TEXT", &run.latency_ms.to_string()),
                binding("TEXT", &run.estimated_cost_usd.map(|value| value.to_string()).unwrap_or_default()),
                binding("TEXT", &run.memory_count.to_string()),
                binding("TEXT", &run.mode),
                binding("TEXT", &run.memory_provider),
                binding("TEXT", &run.measurement_method),
            ],
        )
        .await?;
        Ok(())
    }

    async fn execute(&self, statement: &str, values: Vec<Value>) -> AppResult<Value> {
        let account_url = self.account_url.as_deref().ok_or_else(|| {
            AppError::InvalidInput("Snowflake is not configured. Set SNOWFLAKE_ACCOUNT_URL.".into())
        })?;
        let token = self.token.as_deref().ok_or_else(|| {
            AppError::InvalidInput("Snowflake is not configured. Set a PAT or OAuth token.".into())
        })?;
        let mut bindings = Map::new();
        for (index, value) in values.into_iter().enumerate() {
            bindings.insert((index + 1).to_string(), value);
        }
        let mut body = statement_request(statement, bindings);
        for (key, value) in [
            ("warehouse", self.warehouse.as_deref()),
            ("database", self.database.as_deref()),
            ("schema", self.schema.as_deref()),
            ("role", self.role.as_deref()),
        ] {
            if let Some(value) = value {
                body[key] = Value::String(value.into());
            }
        }
        let response = self
            .http
            .post(format!("{account_url}/api/v2/statements"))
            .bearer_auth(token)
            .header("X-Snowflake-Authorization-Token-Type", &self.token_type)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or_else(|_| json!({}));
        if status.is_success() && status != StatusCode::ACCEPTED {
            Ok(value)
        } else if status == StatusCode::ACCEPTED {
            Err(AppError::Provider(
                "Snowflake query is still running; increase the SQL API timeout or retry.".into(),
            ))
        } else {
            Err(AppError::Provider(
                value["message"]
                    .as_str()
                    .unwrap_or("Snowflake rejected the telemetry request.")
                    .into(),
            ))
        }
    }
}

pub fn local_token_measurement(baseline: &str, optimized: &str) -> TokenMeasurement {
    TokenMeasurement {
        baseline_input_tokens: estimated_tokens(baseline),
        optimized_input_tokens: estimated_tokens(optimized),
        measurement_method: "local_character_estimate".into(),
    }
}

pub fn provider_scaled_measurement(
    baseline: &str,
    optimized: &str,
    actual_input_tokens: Option<i64>,
    mode: &str,
) -> TokenMeasurement {
    let local = local_token_measurement(baseline, optimized);
    let Some(actual) = actual_input_tokens.filter(|value| *value > 0) else {
        return local;
    };
    let (baseline_input_tokens, optimized_input_tokens) = if mode == "baseline" {
        let ratio = local.optimized_input_tokens as f64 / local.baseline_input_tokens.max(1) as f64;
        (actual, (actual as f64 * ratio).round().max(1.0) as i64)
    } else {
        let ratio = local.baseline_input_tokens as f64 / local.optimized_input_tokens.max(1) as f64;
        ((actual as f64 * ratio).round().max(1.0) as i64, actual)
    };
    TokenMeasurement {
        baseline_input_tokens,
        optimized_input_tokens,
        measurement_method: "provider_usage_scaled_estimate".into(),
    }
}

pub fn estimated_tokens(value: &str) -> i64 {
    // Approximate prose compactly while reserving more room for code, dense
    // punctuation, emoji, and non-Latin scripts. Provider tokenizers differ;
    // Bedrock additionally enforces its exact CountTokens preflight.
    let mut prose_chars = 0_i64;
    let mut dense_chars = 0_i64;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character.is_ascii_whitespace() {
            prose_chars += 1;
        } else if character.is_ascii() {
            dense_chars += 1;
        } else {
            dense_chars += 2;
        }
    }
    (((prose_chars + 3) / 4) + ((dense_chars + 1) / 2)).max(1)
}

pub fn estimated_cost_usd(
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_write_input_tokens: Option<i64>,
) -> Option<f64> {
    let input_rate = non_empty_env("KNOWU_INPUT_COST_PER_MILLION")?
        .parse::<f64>()
        .ok()?;
    let output_rate = non_empty_env("KNOWU_OUTPUT_COST_PER_MILLION")?
        .parse::<f64>()
        .ok()?;
    let cache_read_rate = non_empty_env("KNOWU_CACHE_READ_COST_PER_MILLION")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(input_rate);
    let cache_write_rate = non_empty_env("KNOWU_CACHE_WRITE_COST_PER_MILLION")
        .and_then(|value| value.parse::<f64>().ok());
    if cache_write_input_tokens.unwrap_or_default() > 0 && cache_write_rate.is_none() {
        return None;
    }
    Some(
        input_tokens.unwrap_or_default() as f64 * input_rate / 1_000_000.0
            + cache_read_input_tokens.unwrap_or_default() as f64 * cache_read_rate / 1_000_000.0
            + cache_write_input_tokens.unwrap_or_default() as f64
                * cache_write_rate.unwrap_or(input_rate)
                / 1_000_000.0
            + output_tokens.unwrap_or_default() as f64 * output_rate / 1_000_000.0,
    )
}

fn binding(binding_type: &str, value: &str) -> Value {
    json!({"type":binding_type,"value":value})
}

fn statement_request(statement: &str, bindings: Map<String, Value>) -> Value {
    json!({
        "statement":statement,
        "timeout":30,
        "bindings":bindings
    })
}

fn parse_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn optional_number(value: Option<i64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_measurement_is_consistent_and_never_negative() {
        let measurement = local_token_measurement("abcdefgh", "abcd");
        assert_eq!(measurement.baseline_input_tokens, 2);
        assert_eq!(measurement.optimized_input_tokens, 1);
        assert_eq!(measurement.tokens_saved(), 1);
        assert_eq!(measurement.reduction_percent(), 50.0);
    }

    #[test]
    fn provider_usage_scales_the_unsent_comparison_prompt() {
        let measurement = provider_scaled_measurement("abcdefgh", "abcd", Some(10), "optimized");
        assert_eq!(measurement.baseline_input_tokens, 20);
        assert_eq!(measurement.optimized_input_tokens, 10);
        assert_eq!(
            measurement.measurement_method,
            "provider_usage_scaled_estimate"
        );
    }

    #[test]
    fn sql_api_request_omits_unsupported_autocommit_parameter() {
        let request = statement_request("SELECT 1", Map::new());

        assert_eq!(request["statement"], "SELECT 1");
        assert_eq!(request["timeout"], 30);
        assert!(request.get("parameters").is_none());
    }

    #[test]
    fn inference_run_serializes_context_economics_as_camel_case() {
        let run = InferenceRun {
            id: "run-1".into(),
            timestamp: "2026-08-07T12:00:00Z".into(),
            model: "test-model".into(),
            baseline_input_tokens: 100,
            optimized_input_tokens: 60,
            tokens_saved: 40,
            reduction_percent: 40.0,
            actual_input_tokens: Some(62),
            output_tokens: Some(10),
            context_budget_tokens: 80,
            context_estimated_tokens: 55,
            context_units_considered: 12,
            context_units_sent: 8,
            context_units_omitted: 4,
            context_detail_level: "detailed".into(),
            provider_preflight_input_tokens: Some(64),
            cache_read_input_tokens: Some(20),
            cache_write_input_tokens: None,
            latency_ms: 25,
            estimated_cost_usd: Some(0.01),
            memory_count: 3,
            mode: "optimized".into(),
            memory_provider: "local".into(),
            measurement_method: "test".into(),
        };

        let value = serde_json::to_value(run).unwrap();
        assert_eq!(value["contextBudgetTokens"], 80);
        assert_eq!(value["contextUnitsOmitted"], 4);
        assert_eq!(value["providerPreflightInputTokens"], 64);
        assert_eq!(value["cacheReadInputTokens"], 20);
        assert!(value["cacheWriteInputTokens"].is_null());
    }
}
