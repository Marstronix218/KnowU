# KnowU privacy model

KnowU is local-first, not fully local. Detailed activity remains on the Mac;
approved memories, selected AI context, and aggregate telemetry cross explicit
external boundaries.

## Raw local activity

The existing collectors can store foreground app names, permitted window
titles, timestamps, durations, selected Chrome history metadata, and Chrome
active-tab metadata. These records live in the local SQLite database and are
treated as sensitive.

KnowU does not intentionally collect page bodies, DOM content, form input,
keystrokes, clipboard contents, screenshots, audio, or camera data. The Chrome
extension has no content scripts.

Raw activity rows, complete browser history, window titles, page titles, URLs,
and search queries are never uploaded to EverOS or written to Snowflake.

## EverOS boundary

EverOS receives only a user-approved or derived memory:

- profile summary
- interests
- current projects
- recurring high-level patterns
- explicit user corrections and preferences
- hackathon demo seed memories

The Memory page requires an explicit **Sync approved profile** action for bulk
profile ingestion. A correction is explicit user-authored truth, so saving one
also attempts EverOS sync after the local save succeeds.

KnowU uses a dedicated EverOS session for approved memories. Query-specific
search returns episodes/atomic facts; those selected memories are visible in the
Context Economics UI.

## AI provider boundary

OpenAI or Anthropic receives the active conversation plus one selected context:

- **Full Context:** structured profile, user corrections, aggregated activity
  summary, and optional high-level thread brief.
- **KnowU Context:** query-specific approved memories from EverOS (or clearly
  labeled approved local fallback) and optional high-level thread brief.

Neither mode attaches raw activity rows. OpenAI requests set `store: false`.
Provider-side processing and retention remain governed by the user's provider
account and current provider terms.

## Snowflake boundary

By default, Snowflake receives numeric inference telemetry only:

- query/run UUID (not the query text)
- timestamp and model
- baseline/optimized token counts and measurement method
- tokens saved and reduction percentage
- provider input/output usage when available
- latency and optional cost estimate
- memory count, mode, and memory-provider label

KnowU never writes the question, answer, prompt, retrieved memory text, raw
activity, URLs, or titles into `INFERENCE_RUNS`.

Optional `SNOWFLAKE_ENABLE_AI_COUNT_TOKENS=true` submits the two approved,
derived comparison prompts to Snowflake `AI_COUNT_TOKENS`. This is off by
default and should be enabled only when that expanded boundary is explicitly
acceptable.

## Credentials

- OpenAI/Anthropic keys can be stored in macOS Keychain service
  `com.knov.desktop.llm` (a retained compatibility identifier) or supplied by
  environment during source development.
- EverOS and Snowflake credentials are read from the native process environment
  and are never returned to the frontend.
- KnowU does not auto-load `.env`; `.env` files are gitignored.
- Secrets must never be committed.

## Retention and deletion

- Detailed local activity is retained for a rolling 30 days.
- Temporary 31–90 day Chrome bootstrap data is deleted after the first
  successful profile refresh.
- Profiles, corrections, and local inference telemetry remain until local
  deletion.
- **Delete local KnowU data** clears app-owned SQLite rows, resets settings,
  removes provider credentials, rotates pairing state, and removes the local
  Native Messaging manifest.
- Local deletion does not delete EverOS memories. Delete those through EverOS
  account/API controls when required.
- Snowflake telemetry follows the retention policy configured in the user's
  Snowflake account.

SQLite/WAL pages, APFS snapshots, backups, SSD behavior, provider-held request
data, EverOS data, and Snowflake data are outside the forensic guarantees of the
local delete action.
