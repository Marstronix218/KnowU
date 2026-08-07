# KnowU privacy model

KnowU is local-first, not fully local. Detailed activity remains on the Mac;
approved memories, selected AI context, and aggregate telemetry cross explicit
external boundaries.

## Raw local activity

The existing collectors can store foreground app names, permitted window
titles, timestamps, durations, selected Chrome history metadata, Chrome
active-tab metadata, and metadata-only save signals from supported VS
Code-family Local History indexes. When those signals are absent, KnowU can
derive recent workspace-relative changed paths from local Git working-tree
metadata. Editor save records live in local SQLite; Git-derived paths are added
to local UI responses and are treated as sensitive.

KnowU does not intentionally collect page bodies, DOM content, form input,
keystrokes, clipboard contents, screenshots, audio, camera data, source-file
contents, or editor Local History snapshots. The Chrome extension has no
content scripts. Editor metadata collection also excludes hidden paths, common
generated/dependency trees, and credential/key filenames.

The selected thread may show a media preview for a recognized YouTube URL.
KnowU fetches only that video's public thumbnail from an allowlisted YouTube
image host and renders it from memory. The YouTube player is not contacted or
loaded until the user explicitly presses **Play preview**. Generic websites are
not embedded and their page bodies are not fetched for preview; the user can
open the original resource explicitly. Thumbnail and playback requests expose
ordinary network metadata and the selected video ID to YouTube under the
user's current network and provider policies.

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
Context Economics UI. The search request contains the current question and,
after **Ask with context**, the selected thread subject so follow-up retrieval
stays thread-aware. Selected titles, resources, searches, timestamps, and event
metadata are not sent to EverOS.

## AI provider boundary

OpenAI, Anthropic, or Amazon Bedrock receives the bounded active conversation
plus one query-complete context:

- query-specific approved memories from EverOS (or clearly labeled approved
  local fallback)
- compact activity facts computed locally, such as matched count, first/last
  observation, de-overlapped live duration, and source-specific evidence counts
- a token-budgeted set of sanitized event metadata from a thread explicitly
  selected with **Ask with context**

The selected-thread packet can contain page/window titles, search phrases,
domain/path-only resources, timestamps, source/app labels, and reliable live
duration. The native core strips URL query strings/fragments, suppresses
credential-like fields, limits field lengths, and never attaches browser
profile IDs, event IDs, source-file contents, or full raw database rows. This is
a deliberate richer provider boundary than the default aggregate-only chat.

The deterministic packer preserves the thread subject, aggregate facts,
approved memories, source diversity, and caveats before adding more event
evidence until `KNOWU_CONTEXT_TOKEN_BUDGET` is reached. The larger Full Context
payload is never sent to the AI provider; optional Snowflake AI token counting
may receive both derived comparison prompts. OpenAI
requests set `store: false`; Bedrock requests use direct API-key authentication,
model-specific token preflight, and eligible five-minute prompt caching.
Provider-side processing and retention remain governed by the user's provider
account and current provider terms.

When the user explicitly chooses **Ask with context**, the visible local preview
lists the candidate evidence. After the answer, **System context (sent)** shows
the exact packed system context supplied to the provider; the bounded
conversation and current question are separate provider message fields.
Browser-history
duration remains omitted because it is not reliable foreground time.

## Snowflake boundary

By default, Snowflake receives numeric inference telemetry only:

- query/run UUID (not the query text)
- timestamp and model
- baseline/optimized token counts and measurement method
- tokens saved and reduction percentage
- provider input/output usage when available
- context budget/utilization and units considered, included, and omitted
- Bedrock preflight, cache-read, and cache-write token counts
- latency and optional cost estimate
- memory count, mode, and memory-provider label

KnowU never writes the question, answer, prompt, retrieved memory text, raw
activity, URLs, or titles into `INFERENCE_RUNS`.

Optional `SNOWFLAKE_ENABLE_AI_COUNT_TOKENS=true` submits the two approved,
derived comparison prompts to Snowflake `AI_COUNT_TOKENS`. This is off by
default and should be enabled only when that expanded boundary is explicitly
acceptable.

## Credentials

- OpenAI/Anthropic/Amazon Bedrock keys can be stored in the macOS Keychain service
  `com.knowu.desktop.llm` or supplied by the environment during source
  development.
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
