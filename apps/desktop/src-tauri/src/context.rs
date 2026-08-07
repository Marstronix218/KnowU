use chrono::{TimeZone, Utc};
use serde_json::Value;

use crate::{
    memory::MemoryRecord,
    models::{ProfileDocument, QueryActivityFacts, UserCorrection},
};

const SAFETY_BOUNDARY: &str = "Never reveal raw activity logs. Avoid medical or mental-health diagnosis and productivity scoring. Treat explicit user memories as authoritative. Distinguish observations from inferences.";

pub fn build_baseline_context(
    profile: &ProfileDocument,
    corrections: &[UserCorrection],
    activity_summary: &Value,
    memories: &[MemoryRecord],
    activity_facts: &[QueryActivityFacts],
    thread_brief: Option<&str>,
) -> String {
    let selected = format_memories(memories);
    let activity_section = format_activity_section(activity_facts);
    format!(
        "You are KnowU, a supportive personal AI agent. {SAFETY_BOUNDARY}\n\
         BASELINE MODE: use the larger local context below. applicationTime and liveWebsiteTime are observed foreground durations. historicalWebsiteVisits contains visit counts only. recentEditorChanges contains metadata-only editor save signals, never file contents.\n\
         PROFILE:\n{}\nAUTHORITATIVE USER TRUTH:\n{}\nQUERY-SPECIFIC APPROVED MEMORIES:\n{selected}\nLOCAL ACTIVITY SUMMARY:\n{}{activity_section}{}",
        serde_json::to_string(profile).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(corrections).unwrap_or_else(|_| "[]".into()),
        activity_summary,
        optional_thread_brief(thread_brief),
    )
}

pub fn build_optimized_context(
    memories: &[MemoryRecord],
    activity_facts: &[QueryActivityFacts],
    thread_brief: Option<&str>,
) -> String {
    let selected = format_memories(memories);

    let activity_section = format_activity_section(activity_facts);

    format!(
        "You are KnowU, a supportive personal AI agent. {SAFETY_BOUNDARY}\n\
         KNOWU MODE: answer using query-specific approved memories, compact locally derived activity facts, and the active conversation. Do not invent missing personal context.\n\
         RELEVANT MEMORIES:\n{selected}{activity_section}{}",
        optional_thread_brief(thread_brief),
    )
}

fn format_memories(memories: &[MemoryRecord]) -> String {
    let selected = memories
        .iter()
        .map(|memory| {
            format!(
                "- [{} | {}] {}",
                memory.memory_type, memory.source, memory.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if selected.is_empty() {
        "- No relevant approved memories were available.".into()
    } else {
        selected
    }
}

fn format_activity_section(activity_facts: &[QueryActivityFacts]) -> String {
    let activity_facts = activity_facts
        .iter()
        .map(|facts| {
            format!(
                "- subject={}; match basis={}; matched events={}; first seen={}; last seen={}; calendar span seconds={}; observed active seconds={}; app-focus seconds={}; live-browser seconds={}; historical visits={}; browser-history reported seconds={}; editor save signals={}; coverage={} to {}",
                facts.subject,
                facts.match_basis,
                facts.matched_events,
                formatted_timestamp(facts.first_seen_at),
                formatted_timestamp(facts.last_seen_at),
                facts.observed_span_seconds,
                facts.observed_active_seconds,
                facts.app_focus_seconds,
                facts.live_browser_seconds,
                facts.historical_visits,
                facts.historical_reported_seconds,
                facts.editor_changes,
                formatted_timestamp(facts.coverage_start_at),
                formatted_timestamp(facts.coverage_end_at),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if activity_facts.is_empty() {
        String::new()
    } else {
        format!(
            "\nQUERY-SPECIFIC LOCAL ACTIVITY FACTS (compact aggregates; no raw events):\n{activity_facts}\n\
             INTERPRETATION: calendar span is the interval from first to last observed matching activity, not active time. observed active seconds is the non-overlapping union of live app/browser intervals. App-focus and live-browser totals may overlap. Browser-history reported seconds are secondary metadata and are not reliable focused work time. Facts cover only locally retained data, normally 30 days. When asked how long, report span and active-time evidence separately with these caveats; do not claim the information is unavailable when these facts answer it. Subject matching is a query-term inference, not confirmed intent."
        )
    }
}

fn formatted_timestamp(value: i64) -> String {
    Utc.timestamp_opt(value, 0)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn compose_measured_prompt(system: &str, conversation: &str, question: &str) -> String {
    format!("SYSTEM:\n{system}\nCONVERSATION:\n{conversation}\nQUESTION:\n{question}")
}

fn optional_thread_brief(thread_brief: Option<&str>) -> String {
    thread_brief
        .filter(|brief| !brief.trim().is_empty())
        .map(|brief| format!("\nSELECTED HIGH-LEVEL THREAD BRIEF:\n{}", brief.trim()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::estimated_tokens;
    use crate::memory::MemoryRecord;
    use crate::models::QueryActivityFacts;

    #[test]
    fn optimized_context_contains_only_selected_memories() {
        let context = build_optimized_context(
            &[MemoryRecord {
                id: "1".into(),
                text: "Privacy matters more than feature count.".into(),
                memory_type: "preference".into(),
                source: "explicit_user".into(),
                created_at: 0,
                importance: Some(1.0),
                score: Some(0.9),
            }],
            &[QueryActivityFacts {
                subject: "Snowflake".into(),
                match_basis: "exact query-term metadata match".into(),
                matched_events: 220,
                first_seen_at: 100,
                last_seen_at: 400,
                observed_span_seconds: 300,
                observed_active_seconds: 120,
                app_focus_seconds: 120,
                live_browser_seconds: 60,
                historical_visits: 200,
                historical_reported_seconds: 50_000,
                editor_changes: 2,
                coverage_start_at: 0,
                coverage_end_at: 500,
            }],
            None,
        );

        assert!(context.contains("Privacy matters more"));
        assert!(!context.contains("LOCAL ACTIVITY SUMMARY"));
        assert!(context.contains("QUERY-SPECIFIC LOCAL ACTIVITY FACTS"));
        assert!(context.contains("Snowflake"));
        assert!(context.contains("matched events=220"));
        assert!(context.contains("calendar span is the interval"));
        assert!(context.contains("not reliable focused work time"));
        assert!(!context.contains("windowTitle"));
        assert!(context.len() < 1_600);
    }

    #[test]
    fn optimized_context_omits_activity_section_without_a_query_match() {
        let context = build_optimized_context(&[], &[], None);

        assert!(!context.contains("QUERY-SPECIFIC LOCAL ACTIVITY FACTS"));
        assert!(context.contains("No relevant approved memories"));
    }

    #[test]
    fn query_complete_context_remains_smaller_than_a_large_baseline() {
        let profile = ProfileDocument {
            summary: "A detailed but approved profile summary. ".repeat(20),
            interests: (0..20).map(|index| format!("Interest {index}")).collect(),
            skills: (0..20).map(|index| format!("Skill {index}")).collect(),
            active_projects: (0..20).map(|index| format!("Project {index}")).collect(),
            patterns: (0..20).map(|index| format!("Pattern {index}")).collect(),
            updated_at: 0,
        };
        let baseline = build_baseline_context(
            &profile,
            &[],
            &serde_json::json!({
                "today": {"applicationTime": (0..30).map(|index| serde_json::json!({"name":format!("App {index}"),"seconds":index * 60})).collect::<Vec<_>>()},
                "7d": {"historicalWebsiteVisits": (0..30).map(|index| serde_json::json!({"domain":format!("site{index}.example"),"visits":index})).collect::<Vec<_>>()},
                "30d": {"trackedSeconds": 50_000}
            }),
            &[MemoryRecord {
                id: "1".into(),
                text: "Snowflake is an active project.".into(),
                memory_type: "project".into(),
                source: "everos".into(),
                created_at: 0,
                importance: Some(0.8),
                score: Some(0.9),
            }],
            &[QueryActivityFacts {
                subject: "Snowflake".into(),
                match_basis: "exact query-term metadata match".into(),
                matched_events: 220,
                first_seen_at: 100,
                last_seen_at: 400,
                observed_span_seconds: 300,
                observed_active_seconds: 120,
                app_focus_seconds: 120,
                live_browser_seconds: 60,
                historical_visits: 200,
                historical_reported_seconds: 50_000,
                editor_changes: 2,
                coverage_start_at: 0,
                coverage_end_at: 500,
            }],
            None,
        );
        let optimized = build_optimized_context(
            &[MemoryRecord {
                id: "1".into(),
                text: "Snowflake is an active project.".into(),
                memory_type: "project".into(),
                source: "everos".into(),
                created_at: 0,
                importance: Some(0.8),
                score: Some(0.9),
            }],
            &[QueryActivityFacts {
                subject: "Snowflake".into(),
                match_basis: "exact query-term metadata match".into(),
                matched_events: 220,
                first_seen_at: 100,
                last_seen_at: 400,
                observed_span_seconds: 300,
                observed_active_seconds: 120,
                app_focus_seconds: 120,
                live_browser_seconds: 60,
                historical_visits: 200,
                historical_reported_seconds: 50_000,
                editor_changes: 2,
                coverage_start_at: 0,
                coverage_end_at: 500,
            }],
            None,
        );

        assert!(estimated_tokens(&optimized) < estimated_tokens(&baseline));
        assert!(baseline.contains("Snowflake is an active project."));
        assert!(optimized.contains("Snowflake is an active project."));
    }
}
