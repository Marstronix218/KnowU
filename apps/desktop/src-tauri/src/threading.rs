use std::collections::{HashMap, HashSet};

use url::Url;

use crate::models::ActivityEvent;

#[derive(Default)]
struct AnchorStats {
    event_count: usize,
    apps: HashSet<String>,
}

/// Assigns a stable, human-readable subject to events that share a meaningful
/// anchor across two or more signals. This intentionally stays local: it uses
/// metadata already stored by KnowU and never opens page or file contents.
pub fn semantic_topics(events: &[ActivityEvent]) -> HashMap<i64, String> {
    let event_tokens = events
        .iter()
        .map(|event| (event.id, subject_tokens(event)))
        .collect::<Vec<_>>();
    let mut stats = HashMap::<String, AnchorStats>::new();

    for (event, (_, tokens)) in events.iter().zip(&event_tokens) {
        for token in tokens {
            let entry = stats.entry(token.clone()).or_default();
            entry.event_count += 1;
            entry.apps.insert(event.app_name.to_ascii_lowercase());
        }
    }

    let mut assignments = HashMap::new();
    for (event_id, tokens) in event_tokens {
        let Some(event_id) = event_id else {
            continue;
        };
        let anchor = tokens
            .into_iter()
            .filter(|token| stats.get(token).is_some_and(|value| value.event_count >= 2))
            .max_by(|left, right| anchor_score(left, &stats).cmp(&anchor_score(right, &stats)));
        if let Some(anchor) = anchor {
            assignments.insert(event_id, display_label(&anchor));
        }
    }
    assignments
}

fn anchor_score(token: &str, stats: &HashMap<String, AnchorStats>) -> (usize, usize, usize) {
    let value = &stats[token];
    (value.event_count, value.apps.len(), token.len())
}

fn subject_tokens(event: &ActivityEvent) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let mut descriptive_text = String::new();
    for value in [
        event.search_query.as_deref(),
        event.page_title.as_deref(),
        event.window_title.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        extend_tokens(&mut tokens, value);
        descriptive_text.push(' ');
        descriptive_text.push_str(&value.to_ascii_lowercase());
    }
    if let Some(url) = event
        .url
        .as_deref()
        .and_then(|value| Url::parse(value).ok())
    {
        if let Some(host) = url.host_str() {
            extend_tokens(&mut tokens, host);
        }
        extend_tokens(&mut tokens, url.path());
    }
    if tokens.contains("snowflake") && is_natural_snowflake_context(&descriptive_text) {
        tokens.remove("snowflake");
    }
    tokens
}

pub fn is_natural_snowflake_context(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    ["crystal", "photography", "snow storm", "weather", "winter"]
        .iter()
        .any(|signal| value.contains(signal))
}

fn extend_tokens(tokens: &mut HashSet<String>, value: &str) {
    let mut current = String::new();
    for character in value.chars() {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else {
            push_token(tokens, &mut current);
        }
    }
    push_token(tokens, &mut current);
}

fn push_token(tokens: &mut HashSet<String>, current: &mut String) {
    if current.len() >= 4
        && !is_stopword(current)
        && !current.chars().all(|value| value.is_numeric())
    {
        tokens.insert(std::mem::take(current));
    } else {
        current.clear();
    }
}

fn is_stopword(value: &str) -> bool {
    matches!(
        value,
        "about"
            | "account"
            | "activity"
            | "application"
            | "architecture"
            | "beginner"
            | "browser"
            | "chrome"
            | "client"
            | "cloud"
            | "code"
            | "com"
            | "dashboard"
            | "data"
            | "desktop"
            | "developer"
            | "development"
            | "document"
            | "docs"
            | "file"
            | "getting"
            | "google"
            | "guide"
            | "home"
            | "html"
            | "http"
            | "https"
            | "implementation"
            | "index"
            | "introduction"
            | "learn"
            | "main"
            | "notes"
            | "official"
            | "overview"
            | "page"
            | "platform"
            | "pricing"
            | "project"
            | "query"
            | "research"
            | "results"
            | "schema"
            | "search"
            | "software"
            | "studio"
            | "tutorial"
            | "using"
            | "video"
            | "watch"
            | "window"
            | "with"
            | "work"
            | "workspace"
            | "youtube"
    )
}

fn display_label(anchor: &str) -> String {
    match anchor {
        "bigquery" => "BigQuery".into(),
        "databricks" => "Databricks".into(),
        "openai" => "OpenAI".into(),
        "postgresql" => "PostgreSQL".into(),
        "snowflake" => "Snowflake".into(),
        _ => {
            let mut characters = anchor.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ActivitySource;

    fn event(
        id: i64,
        app: &str,
        page_title: Option<&str>,
        window_title: Option<&str>,
        url: Option<&str>,
        search_query: Option<&str>,
    ) -> ActivityEvent {
        ActivityEvent {
            id: Some(id),
            occurred_at: id,
            ended_at: None,
            duration_seconds: 0,
            app_name: app.into(),
            window_title: window_title.map(str::to_string),
            url: url.map(str::to_string),
            page_title: page_title.map(str::to_string),
            search_query: search_query.map(str::to_string),
            browser_profile_id: None,
            source: ActivitySource::AppFocus,
            is_bootstrap: false,
        }
    }

    #[test]
    fn groups_snowflake_across_search_video_dashboard_document_and_editor() {
        let events = vec![
            event(
                1,
                "Google Chrome",
                Some("Snowflake architecture tutorial - YouTube"),
                None,
                Some("https://youtube.com/watch?v=demo"),
                None,
            ),
            event(
                2,
                "Google Chrome",
                Some("snowflake architecture - Google Search"),
                None,
                Some("https://google.com/search?q=snowflake"),
                Some("snowflake architecture"),
            ),
            event(
                3,
                "Google Chrome",
                Some("Snowsight"),
                None,
                Some("https://app.snowflake.com/example"),
                None,
            ),
            event(
                4,
                "Preview",
                Some("Snowflake migration notes.pdf"),
                None,
                None,
                None,
            ),
            event(
                5,
                "Cursor",
                Some("src/snowflake_client.rs"),
                Some("KnowU — src/snowflake_client.rs"),
                None,
                None,
            ),
        ];

        let topics = semantic_topics(&events);

        assert_eq!(topics.len(), 5);
        assert!(topics.values().all(|value| value == "Snowflake"));
    }

    #[test]
    fn keeps_unrelated_warehouse_work_separate() {
        let events = vec![
            event(1, "Chrome", Some("Snowflake pricing"), None, None, None),
            event(
                2,
                "Preview",
                Some("Snowflake migration.md"),
                None,
                None,
                None,
            ),
            event(3, "Chrome", Some("BigQuery pricing"), None, None, None),
            event(4, "Cursor", Some("bigquery_client.ts"), None, None, None),
        ];

        let topics = semantic_topics(&events);

        assert_eq!(topics.get(&1).map(String::as_str), Some("Snowflake"));
        assert_eq!(topics.get(&2).map(String::as_str), Some("Snowflake"));
        assert_eq!(topics.get(&3).map(String::as_str), Some("BigQuery"));
        assert_eq!(topics.get(&4).map(String::as_str), Some("BigQuery"));
    }

    #[test]
    fn does_not_merge_weather_snowflakes_with_the_company() {
        let events = vec![
            event(
                1,
                "Chrome",
                Some("How a snowflake forms in winter"),
                None,
                None,
                None,
            ),
            event(
                2,
                "Chrome",
                Some("Snowflake data warehouse"),
                None,
                Some("https://app.snowflake.com"),
                None,
            ),
        ];

        assert!(semantic_topics(&events).is_empty());
    }
}
