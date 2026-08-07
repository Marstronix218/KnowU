use serde_json::Value;

use crate::{
    memory::MemoryRecord,
    models::{ProfileDocument, UserCorrection},
};

const SAFETY_BOUNDARY: &str = "Never reveal raw activity logs. Avoid medical or mental-health diagnosis and productivity scoring. Treat explicit user memories as authoritative. Distinguish observations from inferences.";

pub fn build_baseline_context(
    profile: &ProfileDocument,
    corrections: &[UserCorrection],
    activity_summary: &Value,
    thread_brief: Option<&str>,
) -> String {
    format!(
        "You are KnowU, a supportive personal AI agent. {SAFETY_BOUNDARY}\n\
         BASELINE MODE: use the larger local context below. applicationTime and liveWebsiteTime are observed foreground durations. historicalWebsiteVisits contains visit counts only.\n\
         PROFILE:\n{}\nAUTHORITATIVE USER TRUTH:\n{}\nLOCAL ACTIVITY SUMMARY:\n{}{}",
        serde_json::to_string(profile).unwrap_or_else(|_| "{}".into()),
        serde_json::to_string(corrections).unwrap_or_else(|_| "[]".into()),
        activity_summary,
        optional_thread_brief(thread_brief),
    )
}

pub fn build_optimized_context(memories: &[MemoryRecord], thread_brief: Option<&str>) -> String {
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
    let selected = if selected.is_empty() {
        "- No relevant approved memories were available.".into()
    } else {
        selected
    };

    format!(
        "You are KnowU, a supportive personal AI agent. {SAFETY_BOUNDARY}\n\
         KNOWU MODE: answer using only the query-specific approved memories below plus the active conversation. Do not invent missing personal context.\n\
         RELEVANT MEMORIES:\n{selected}{}",
        optional_thread_brief(thread_brief),
    )
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
    use crate::memory::MemoryRecord;

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
            None,
        );

        assert!(context.contains("Privacy matters more"));
        assert!(!context.contains("LOCAL ACTIVITY SUMMARY"));
    }
}
