# Architecture

## Scope

KnowU is a single-user, local-first macOS application. React renders the
interface, Rust owns native and security-sensitive operations, SQLite stores
app-owned data, and a Chrome extension supplies accurate active-tab timing.
EverOS supplies approved persistent memory, while Snowflake receives aggregate
token-economics telemetry. Raw personal activity remains local.

## Components

| Component | Location | Responsibility |
| --- | --- | --- |
| React/Vite interface | `apps/desktop/src` | Onboarding, dashboard, history, profile, assistant, and settings |
| Tauri/Rust core | `apps/desktop/src-tauri/src` | IPC commands, collection, continuous Chrome backfill, editor-history metadata import, retention, context assembly, SQLite, Keychain, scheduling, and provider calls |
| Memory service | `apps/desktop/src-tauri/src/memory` | Approved-memory ingestion, EverOS v1 add/flush/search, and safe local fallback |
| Context builders | `apps/desktop/src-tauri/src/context.rs` | Deterministic token-budgeted packing of memories, aggregates, and sanitized selected-thread evidence |
| Analytics service | `apps/desktop/src-tauri/src/analytics` | Token measurement provenance and Snowflake SQL API telemetry |
| SQLite store | Tauri application-data directory | Activity, settings, profiles, corrections, recommendations, inference telemetry, and extension pairing state |
| Chrome extension | `apps/extension` | Active-tab URL/title timing, exclusions, pause, and local transport |
| Native Messaging helper | `apps/desktop/src-tauri/src/bin/knowu-native-host.rs` | Chrome stdio framing and forwarding to the running Rust core |
| OpenAI, Anthropic, or Amazon Bedrock | external | Profile generation, recommendations, and assistant responses; Bedrock adds exact token preflight and prompt-cache usage |
| EverMind EverOS | external | Persistent approved memory and query-specific retrieval |
| Snowflake | external | Aggregate inference-run telemetry and context-economics analysis |

No Swift helper is currently used.

## Runtime flow

```text
macOS foreground app/window
            |
            v
      Rust collector ----------------------+
                                            |
selected Chrome History --> temporary copy  |
VS Code-family Local History indexes -------+
                                            v
Chrome tabs --> extension --> local bridge --> SQLite
                                            |
                      aggregate/redact/domain-only digest
                                            |
                                            v
                                  approved profile facts ----> EverOS
                                            |
current question + selected subject -> relevant memory search
       |                                    |
       +--> local query-specific facts -----+
       +--> sanitized selected evidence ----+
                                            |
          deterministic token-budgeted context packer
                                            |
                                            v
                           OpenAI / Anthropic / Bedrock
                                            |
                                            +----> answer + provider usage
                                            |
                                            v
                              local inference telemetry ----> Snowflake

larger full-context baseline --> local comparison; optional Snowflake token count
```

The answer path always uses relevant memories plus compact local facts. An
explicitly selected thread also contributes sanitized titles, searches,
domain/path resources, timestamps, and reliable live durations. Units are
ranked deterministically under a configurable token budget; authoritative and
aggregate evidence is kept before representative event detail. The larger
baseline is not an alternate answer mode and is never sent to the AI provider;
it demonstrates the savings without omitting answer-critical facts. When the
explicit `SNOWFLAKE_ENABLE_AI_COUNT_TOKENS` option is enabled, Snowflake receives
both derived comparison prompts solely to count tokens.

The frontend calls typed Tauri commands through `invoke`. It does not open the
database, read Keychain, or call providers directly. Outside Tauri, the same
frontend returns explicit mock data for design and browser tests.

## Collection

The Rust collector samples every five seconds by default. On macOS it invokes
`System Events` through `osascript` to identify the frontmost application and,
when Accessibility permission permits it, the front-window title. A continuous
session is stored when the app or title changes.

Chrome history import:

1. Discovers profiles under
   `~/Library/Application Support/Google/Chrome`.
2. Requires the user to select at least one profile.
3. Copies each selected `History` database to a uniquely named temporary file.
4. Reads visits from the previous 90 days.
5. Deletes the temporary copy after the import attempt.

While collection is active, the Rust core checks selected Chrome `History`
databases approximately every 30 seconds. When one changes, it repeats a
deduplicated two-day backfill so new visits appear without another manual
import. These rows are visit metadata, not reliable active-tab duration.

VS Code-family editor context does not require an extension. Every 30 seconds,
KnowU checks the Local History indexes for Visual Studio Code, Cursor, and
Cortex Code. It records the editor, workspace-relative file path, and save
timestamp for recent files inside known workspaces. It never opens the saved
snapshot or current source file. Hidden files, common generated/dependency
trees, and credential/key filenames are excluded. If Local History and window
titles do not identify a file, dashboard and activity responses may also derive
recent changed paths from Git working-tree metadata in the editor's most
recently active local workspace; source contents are still never opened.

Work threads are grouped by subject rather than application category. The Rust
core extracts repeated local anchors from search queries, page/window titles,
domains, document names, and editor-relative paths. For example, Snowflake
signals from YouTube, a search page, a document, `app.snowflake.com`, and an
editor filename receive the same `Snowflake` topic while their original events
remain separate evidence. Common intent aliases can also resolve to a cautious
canonical subject—for example, workouts, plank challenges, sit-ups, and
strength training resolve to `Workout`. A low-information search or video
navigation page inherits that subject only when nearby activity in the same
known browser profile is unambiguous. This immediate grouping is deterministic
and local.
An optional future LLM refinement should operate only on bounded extracted
terms, domains, counts, and source types—not raw activity rows—and must fall
back to these local assignments if the provider is unavailable.

Visits older than 30 days are flagged as temporary bootstrap data. They remain
until the first profile succeeds, then are deleted.

The extension observes the active HTTP(S) tab while Chrome is focused. It stores
only the unfinished session in `chrome.storage.session`. Completed events receive
one delivery attempt and are not persisted or retried. The extension does not
use content scripts. Each installation is configured with an approved native
Chrome profile ID, which the ingestion core enforces.

## Local extension transports

Native Messaging is the intended transport. Chrome starts
`com.knowu.companion`, which forwards framed messages over a mode-0600 Unix
domain socket in the app-data directory. The Rust core validates protocol
version, pairing token, and extension ID before accepting events.

A loopback HTTP transport exists for development. It accepts only loopback HTTP
endpoints and requires a bearer pairing token. It is not the production
transport and does not provide TLS.

Source builds require loading the extension unpacked and building the helper.
Settings exposes host registration and the pairing token. See
[Alpha setup](alpha-setup.md).

## Storage

The Rust core is the only SQLite writer. On macOS, Tauri resolves the database
under its application-data directory, normally:

```text
~/Library/Application Support/com.knowu.desktop/knowu.sqlite3
```

SQLite runs in WAL mode. The schema uses `PRAGMA user_version` migrations.
Provider keys are not stored in SQLite; Keychain entries use service
`com.knowu.desktop.llm` and provider account names `openai` or `anthropic`.

Main stored records:

- detailed app, window, URL, page-title, and search-query events
- selected Chrome profiles and collection settings
- generated profile versions and recommendations
- separately stored authoritative user corrections
- pairing token, first authenticated extension ID, and last-seen timestamp

Chat messages are held in frontend memory for the current session and are not
persisted by KnowU.

## Profiling and scheduling

Profile refresh produces a local aggregate containing at most 200 grouped
activity entries. Each entry includes app name, a truncated locally redacted
title, domain only, accumulated seconds, and occurrence count.
The digest and authoritative corrections go directly to the selected provider.

The scheduler checks once per minute and attempts one refresh per local calendar
day when a provider and credential are available. This also provides catch-up
after sleep or restart. Manual refresh uses the same provider path. A successful
first refresh deletes bootstrap activity older than 30 days.

## Current implementation boundaries

- Chrome is implemented; Safari and Firefox are not.
- Extension-free editor context reflects Local History saves and recent Git
  working-tree paths, not unsaved edits, terminal commands, cursor position,
  selections, or diagnostics.
- Native helper registration is not a polished installer flow.
- Extension exclusions are stored separately; the desktop collection state is
  synchronized on extension status checks.
- Launch at login is persisted locally and applied through the Tauri autostart
  plugin.
- Behavioral guidance is suppressed during generation and dashboard display
  when disabled.
- Provider-key removal is available in Settings.
- Profile summary editing, inferred-item suppression, and editable authoritative
  corrections are available locally.
- The dashboard derives top-application, longest-session, distinct-page, and
  cautious local topic/category insights; provider recommendations are also
  implemented.
- The bundled frontend uses a restrictive Tauri content security policy.
