# KnowU

**Remembers more. Sends less.**

KnowU is a memory-efficient personal AI agent built for the Snowflake × Beta
Fund × EverMind Agent & Token Economy Hackathon, targeting Track 1: Cost of
Intelligence.

KnowU learns useful facts about a user over time, stores approved persistent
memories in EverMind EverOS, retrieves only memories relevant to the current
question, and records the resulting context economics in Snowflake.

## The problem

Personal agents become more useful as they learn about a user, but the common
approach—sending an ever-growing profile, history, and conversation on every
request—makes each answer progressively more expensive. Context cost grows even
when most of that context is irrelevant to the current question.

## The solution

For each question, KnowU builds two real context payloads while using one
query-complete answer path:

1. **Full Context** is an unsent comparison baseline: the larger local profile,
   authoritative corrections, summarized activity, conversation, and optional
   thread brief. It includes the same retrieved memories and query-specific
   facts as the answer path, making the token comparison answer-equivalent.
2. **KnowU Context** combines query-specific approved memories from EverOS with
   compact activity facts computed locally from SQLite. It sends facts such as
   first/last observation, de-overlapped live activity time, and evidence
   counts—not raw activity rows.
3. KnowU Context always produces the answer through the configured OpenAI or
   Anthropic BYOK provider path. Full Context is comparison-only, so answer
   correctness never depends on selecting a more expensive mode.
4. KnowU displays the full/optimized input counts, tokens saved, reduction
   percentage, retrieved memories, model, latency, and telemetry status.
5. Aggregate inference-run metrics are persisted locally and synced to
   Snowflake when configured.

The main demo screen makes the flow visible:

```text
QUESTION
   ↓
EVEROS RELEVANT MEMORIES
   +
LOCAL QUERY-SPECIFIC FACTS
   ↓
ANSWER
   +
FULL CONTEXT vs KNOWU CONTEXT TOKEN SAVINGS
```

## Architecture

```text
React/Vite UI
  └─ Tauri commands
      ├─ local SQLite activity/profile store
      ├─ context builders
      │   ├─ baseline: profile + safe summaries
      │   └─ answer: query-specific memories + compact local facts
      ├─ MemoryService
      │   └─ EverOSMemoryService → EverOS Memory API v2
      ├─ OpenAI / Anthropic BYOK provider
      └─ SnowflakeAnalyticsService → Snowflake SQL API
          ├─ INFERENCE_RUNS
          └─ CONTEXT_ECONOMICS_SUMMARY
```

Key implementation locations:

- `apps/desktop/src-tauri/src/memory/mod.rs` — memory abstraction, EverOS v2
  add/flush/search, safe local fallback, profile ingestion, and demo seed.
- `apps/desktop/src-tauri/src/context.rs` — baseline and optimized builders.
- `apps/desktop/src-tauri/src/analytics/mod.rs` — token measurement and
  Snowflake SQL API telemetry.
- `apps/desktop/src-tauri/src/commands.rs` — chat orchestration and context
  assembly.
- `apps/desktop/src/App.tsx` — Context Economics, retrieved-memory disclosure,
  comparison previews, and memory sync controls.
- `snowflake/setup.sql` — Snowflake table and aggregate view.

The app uses KnowU consistently in its product branding and internal
identifiers, including the bundle ID, SQLite filename, native-host name, and
npm workspace scopes.

## Local activity context

While collection is active, KnowU enriches its local timeline from three
sources:

- macOS foreground application and permitted window-title sessions
- selected Chrome profile history, automatically backfilled about every 30
  seconds after the database changes
- metadata-only Local History save signals from Visual Studio Code, Cursor,
  and Cortex Code, without requiring an editor extension

The editor collector stores only the editor, workspace-relative path, and save
timestamp. It never opens source files or saved Local History snapshots. This
extension-free approach cannot see unsaved edits, terminal commands, cursor
movement, selections, or diagnostics.

KnowU also groups repeated subjects across those sources. A Snowflake search,
YouTube tutorial, dashboard, document, and editor path can therefore appear as
one `Snowflake` work thread with each original event shown as evidence. Obvious
shared subjects are matched locally; an LLM is not required for this fast path.

## EverOS integration

KnowU uses the current unified EverOS Memory API v2 by default:

- `POST /api/v2/memory/add` for approved memories
- `POST /api/v2/memory/flush` to trigger extraction
- `POST /api/v2/memory/search` with hybrid retrieval for each query

Legacy EverOS Cloud accounts can set `EVEROS_API_VERSION=v1`. KnowU then uses
the documented v1 personal-memory add, flush, and search contracts while
preserving the same local privacy boundary and safe fallback behavior.

The `MemoryService` interface isolates the application from vendor-specific
logic. `EverOSMemoryService` implements persistent storage and retrieval. If
EverOS is temporarily unavailable, the app still answers using safe derived
local profile memories and labels the run `local-fallback`.

Two explicit ingestion paths are available:

- **Memory → Sync approved profile** sends only the generated summary,
  interests, projects, patterns, and user corrections.
- **Context Economics → Seed demo memories** inserts three safe demo memories
  and flushes them for a reliable presentation.

Adding or editing a user correction always saves locally first and then attempts
to sync that approved correction to EverOS.

## Snowflake integration

KnowU sends one aggregate telemetry row per query through the Snowflake SQL API.
It does **not** write the query, answer, raw history, window titles, URLs, or
memory text into Snowflake.

Each `INFERENCE_RUNS` row contains:

- model and context strategy
- baseline and optimized input tokens
- tokens saved and reduction percentage
- actual provider input/output usage when returned
- latency and optional configured cost estimate
- memory count and memory provider
- measurement method and timestamp

Run [`snowflake/setup.sql`](snowflake/setup.sql) before starting the app with
Snowflake credentials. The included `CONTEXT_ECONOMICS_SUMMARY` view provides
daily totals and average reduction.

Snowflake requires users authenticating with programmatic access tokens to be
covered by a network policy by default. If the SQL API returns error `390432`
(`Network policy is required`), assign an allowlisted network policy to the PAT
user or generate a temporary human-user PAT with Snowflake's network-policy
requirement bypass enabled before retrying.

## Privacy model

Raw activity remains on the Mac. KnowU never uploads raw browser history, raw
app events, window titles, page titles, URLs, or detailed activity rows to
EverOS or Snowflake.

External boundaries:

- **EverOS:** approved/derived memories only—profile facts, interests,
  projects, patterns, explicit corrections, and demo seed facts.
- **AI provider:** the active conversation plus query-specific approved
  memories and compact locally derived activity facts. Raw activity rows are
  never attached.
- **Snowflake:** numeric/aggregate inference telemetry only by default.

Optional Snowflake `AI_COUNT_TOKENS` support is off by default because it would
send the derived comparison prompts to Snowflake for counting. Enable it only
if that approved-data boundary fits the demo policy.

## How token reduction is measured

KnowU serializes the exact baseline and optimized prompts before the provider
call. By default it uses the selected provider's actual input usage for the
prompt that was sent and scales the unsent comparison prompt using the same
deterministic character-based ratio. The UI labels this
`provider usage scaled estimate`.

If the provider omits usage, both prompts use the same transparent local
character estimate. If `SNOWFLAKE_ENABLE_AI_COUNT_TOKENS=true`, both approved
derived prompts are counted with Snowflake `AI_COUNT_TOKENS`; the measurement
method and tokenizer model are recorded with the run. No metric is presented as
actual when it is estimated.

Output tokens come from provider usage when available, with the same local
estimate as a fallback. Cost is calculated only when the two optional current
per-million-token rates are supplied.

## Local development

Requirements:

- Apple Silicon Mac (the collector's tested target)
- Node.js 20.19+ or 22.12+ and npm
- current stable Rust toolchain
- Xcode Command Line Tools
- Chrome 120+ only if using the local companion extension

Install dependencies and validate the project:

```sh
npm install
npm test
npm run typecheck
npm run test:rust
```

The app intentionally does not auto-load `.env`. Copy the example, fill in
credentials, export it into the current shell, and start Tauri:

```sh
cp .env.example .env
set -a
source .env
set +a
npm run dev:desktop
```

Browser-only UI preview (sample data, no native APIs):

```sh
npm run dev
```

Production desktop bundle:

```sh
npm run build:desktop
```

## Environment variables

Required for the complete hackathon demo:

- `EVEROS_API_KEY`
- `EVEROS_USER_ID` (a non-PII application identifier is recommended)
- `OPENAI_API_KEY` or `ANTHROPIC_API_KEY` (or save one in the app Keychain UI)
- `SNOWFLAKE_ACCOUNT_URL`
- `SNOWFLAKE_PROGRAMMATIC_ACCESS_TOKEN` or `SNOWFLAKE_OAUTH_TOKEN`
- `SNOWFLAKE_WAREHOUSE`
- `SNOWFLAKE_DATABASE` (use `KNOWU` with the provided setup)
- `SNOWFLAKE_SCHEMA` (use `ANALYTICS`)
- `SNOWFLAKE_ROLE`

Optional variables and defaults are documented in [`.env.example`](.env.example).
Secrets are ignored by git and must never be committed.

## Demo flow

1. Start KnowU with provider, EverOS, and Snowflake credentials.
2. Open **Ask with context** and confirm both integration indicators are green.
3. Choose **Seed demo memories**. The UI confirms three approved memories were
   written to EverOS and flushed.
4. Ask: **“What should I prioritize when building the production version?”**
5. Show the EverOS memories retrieved, the answer, Full Context tokens, KnowU
   Context tokens, tokens saved, and the reduction percentage.
6. Expand **Compare context payloads** to show exactly why the answer prompt is
   smaller; point out that it contains compact query-specific facts rather than
   raw activity.
7. With Snowflake activity present locally, ask: **“How long have I been working
   on Snowflake?”** Show that the answer separates calendar span, observed live
   time, and historical-visit metadata while Full Context remains comparison-only.
8. Open **Memory → Add correction** and enter: **“Accessibility is more
   important to me than visual polish.”** Save it; KnowU confirms EverOS sync.
9. Return to the assistant and ask: **“What tradeoff should I make next?”** Show
   the new correction retrieved from EverOS and influencing the answer.
10. Show `CONTEXT_ECONOMICS_SUMMARY` in Snowflake to verify aggregate telemetry.

## KnowU capabilities

- KnowU product identity and “Remembers more. Sends less.” experience
- vendor-isolated memory service and EverOS v2 persistent integration
- explicit safe-memory ingestion and deterministic EverOS demo seed
- query-specific selective-context engine and baseline comparison
- provider-usage-aware token economics with visible measurement provenance
- Snowflake inference telemetry schema, SQL API writer, and aggregate view
- Context Economics UI with retrieved memories and inspectable context payloads
- graceful missing-credential and integration-failure behavior

Local collector and Chrome-pairing documentation is available in `docs/`; its
paths, identifiers, and examples use the canonical KnowU naming.
