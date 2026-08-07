# KnowU Product Design

## Product promise

**KnowU remembers more. Sends less.**

KnowU is a memory-efficient personal AI agent. Its primary experience proves
that persistent personalization does not require an indefinitely growing prompt.

## Demo story

The central screen communicates one causal chain:

```text
Question → Relevant EverOS memories → Answer → Token savings
```

The user must be able to understand, without narration:

- which context mode generated the answer;
- which memories were selected;
- how large the full baseline would have been;
- how large the KnowU context was;
- how many tokens and what percentage were saved; and
- whether EverOS retrieval and Snowflake telemetry succeeded.

## Information architecture

- **Now:** local continuity and existing working-context foundation.
- **Threads:** inspectable, provisional work streams derived locally.
- **Memory:** approved facts, explicit corrections, and EverOS sync controls.
- **Activity:** raw local timeline; never an external-memory or telemetry feed.
- **Ask with context:** the hackathon hero experience and Context Economics.
- **Settings:** provider, local collection, browser companion, and deletion.

## Context Economics

The assistant screen uses a two-column desktop layout:

- The main column contains the mode toggle, conversation, and composer.
- The evidence rail contains the reduction headline, full/optimized counts,
  token savings, retrieved memories, context previews, and integration status.

The reduction percentage is the largest metric. Full Context and KnowU Context
must remain simultaneously visible. Token measurement provenance is always
shown; estimates are never styled or described as actual usage.

## Memory learning

Explicit corrections are authoritative and save locally before any external
request. When EverOS is configured, the same approved correction is synced and
can be retrieved by a later, semantically related question. The UI reports local
save success separately from EverOS sync success.

## Privacy language

Use concrete boundaries:

- “Raw activity stays on this Mac.”
- “Only approved profile memories are synced to EverOS.”
- “Snowflake receives aggregate inference telemetry.”
- “KnowU Context contains query-specific memories, not full history.”

Avoid omniscience, surveillance, productivity scoring, or diagnostic language.
Inferences remain provisional; explicit user memories remain authoritative.

## Brand

- Product name: **KnowU**
- Tagline: **Remembers more. Sends less.**
- Visual system: retain the existing dark local-first shell, lime memory accent,
  blue evidence accent, compact typography, and inspectable system status.
- Internal legacy identifiers may remain where changing them risks data,
  migration, Keychain, Tauri, or Chrome Native Messaging compatibility.

## Reliability states

- The app launches without EverOS or Snowflake credentials.
- Missing credentials produce explicit local-fallback status, not a crash.
- Provider failures remain visible in the conversation.
- EverOS empty/error retrieval falls back only to approved derived local memory.
- Snowflake failures persist the run locally and label it local-only.
- Browser preview is visibly sample data and never represents sample metrics as
  native integration results.
