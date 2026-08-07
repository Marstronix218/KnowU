# Design

## Source of truth

- Status: Active
- Last refreshed: 2026-08-07
- Primary product surfaces: Tauri desktop app and Chrome companion extension
- Evidence reviewed: `knowu_prd.md`, `apps/desktop/src/App.tsx`, `apps/desktop/src/App.css`, `apps/extension/src/ui.css`, `docs/screenshots/dashboard.jpg`, and brand assets under `docs/brand/`

## Brand

- Personality: private, calm, precise, technically credible, and quietly helpful.
- Trust signals: visible collection state, explicit local/cloud boundaries, editable memories, and inspectable evidence.
- Avoid: omniscience, surveillance language, productivity scoring, diagnostic claims, and decorative complexity that competes with evidence.
- Product name: **KnowU**
- Tagline: **Remembers more. Sends less.**
- Internal legacy identifiers may remain when renaming risks data, migrations, Keychain, Tauri, or Chrome Native Messaging compatibility.

## Product goals

- Goals: make continuity immediately useful, prove relevant memory can reduce prompt size, and keep privacy boundaries understandable without narration.
- Non-goals: autonomous actions, interruptive coaching, page-body capture, medical or productivity diagnosis, and hidden cloud synchronization.
- Success signals: users understand what KnowU remembered, why an answer is grounded, what stayed local, and how much context was avoided.

## Personas and jobs

- Primary personas: technically capable, privacy-aware, AI-heavy knowledge workers on Apple Silicon Macs.
- User jobs: resume active work, ask context-aware questions, inspect or correct memories, review activity, and control collection.
- Key contexts of use: focused desktop work, quick status checks, assistant conversations, setup, and privacy review.

## Information architecture

- Primary navigation: **Now**, **Threads**, **Memory**, **Activity**, **Settings**, with **Ask with context** as the assistant experience.
- Core routes/screens: continuity overview, provisional thread explorer, approved memory editor, raw local timeline, context-aware assistant, and controls.
- Content hierarchy: current work and next move first; supporting evidence second; raw activity and configuration progressively disclosed.

## Design principles

- Continuity before analytics: lead with what the user can resume, not a dashboard score.
- Evidence before inference: show observed facts and provenance beside provisional conclusions.
- Privacy in the interface: collection, sync, fallback, and deletion states stay visible at the point of use.
- Compact, never microscopic: density is valuable only while labels, controls, metadata, and body copy remain comfortably readable.
- Tradeoffs: preserve desktop information density, but allow secondary text to wrap or rows to grow when readability requires it.

## Visual language

- Color: dark local-first shell, lime memory/action accent, blue evidence accent, and restrained orange/red warning states.
- Typography: system sans-serif with strong display headlines and compact supporting text. Desktop micro labels must be at least 11px, controls and metadata at least 12px, and reading/body copy generally 14px. Uppercase labels use modest tracking so they remain legible at the minimum size.
- Spacing/layout rhythm: dense 4–14px internal rhythm, 18–32px card padding, and generous separation between major sections.
- Shape/radius/elevation: softly rounded panels and controls, restrained borders, and low-contrast elevation.
- Motion: brief state transitions only; respect reduced-motion preferences.
- Imagery/iconography: Lucide icons and the KnowU mark; icons clarify state but do not replace labels.

## Components

- Existing components to reuse: sidebar navigation, status pills, metric cards, evidence rails, thread cards, forms, modals, and assistant messages.
- New/changed components: an activity preview card may show the selected thread's latest resumable resource. YouTube uses a locally mediated thumbnail and click-to-play player; generic sites remain metadata cards with an explicit open action.
- Variants and states: default, hover, focus-visible, selected, disabled, loading, empty, success, warning, offline, and destructive.
- Token/component ownership: shared desktop colors and typography live in `apps/desktop/src/App.css`; extension presentation lives in `apps/extension/src/ui.css`.

## Accessibility

- Target standard: WCAG 2.2 AA for contrast, focus, keyboard access, and readable text.
- Keyboard/focus behavior: all interactive elements retain a visible lime focus ring and logical document order.
- Contrast/readability: avoid rendered text below 11px; use at least 14px for conversation and explanatory copy; muted text must remain distinguishable on dark panels.
- Screen-reader semantics: use native headings, buttons, links, labels, forms, details, and status copy before custom semantics.
- Reduced motion and sensory considerations: honor `prefers-reduced-motion`; never rely on color alone for status.

## Responsive behavior

- Supported breakpoints/devices: desktop app from 720px wide; Chrome popup at 340–360px; wide desktop layouts adapt at 1100px and 860px.
- Layout adaptations: dashboard columns stack, the evidence rail moves below the main assistant, thread grids reduce columns, and headers wrap.
- Touch/hover differences: hover is supplementary; click targets and visible labels carry the interaction.

## Interaction states

- Loading: preserve layout and show explicit progress.
- Empty: explain what is missing and the safe next action.
- Error: keep provider and integration failures visible without exposing credentials.
- Success: distinguish local saves from external sync success.
- Disabled: retain readable labels and explain prerequisites nearby.
- Offline/slow network: local collection and approved local memories continue; external services are labeled unavailable or local-only.
- Media preview: loading preserves the card footprint; thumbnail failure falls back to resource metadata; playback begins only after an explicit user action.

## Content voice

- Tone: concise, concrete, non-judgmental, and transparent about uncertainty.
- Terminology: “Raw activity stays on this Mac,” “Only approved profile memories are synced to EverOS,” “Snowflake receives aggregate inference telemetry,” and “KnowU Context contains query-specific memories, not full history.”
- Microcopy rules: name the data, destination, and state; estimates are never styled or described as actual usage.

## Implementation constraints

- Framework/styling system: React and TypeScript in Tauri 2 with repo-local CSS; Chrome companion uses standalone TypeScript and CSS.
- Design-token constraints: extend the existing CSS variables and selectors; do not add a design-system dependency for local visual changes.
- Performance constraints: keep styling static and dependency-free.
- Compatibility constraints: Apple Silicon macOS alpha and current Chrome extension surface; preserve native integration identifiers. Remote preview media must be provider-allowlisted, click-gated, and must not become a general-purpose browser surface.
- Test/screenshot expectations: run TypeScript checks, tests, production builds, and visually inspect affected surfaces when presentation changes.

## Context Economics

The assistant communicates `Question → Relevant EverOS memories → Answer → Token savings`. Its two-column desktop layout keeps the conversation beside the evidence rail. The reduction percentage remains the largest metric; Full Context and KnowU Context remain simultaneously visible; token provenance is always shown.

Explicit corrections save locally before any external request. When EverOS is configured, approved corrections sync separately, and the interface reports local save and external sync outcomes independently.

## Open questions

- [ ] Validate the refreshed typography scale on the smallest supported 720px desktop window and the 340px Chrome popup before public alpha release.
