# KnowU — pitch cue cards

Deck: `KnowU_Hackathon_Demo_v2.pptx` · 10 slides · ~4 min 45 s + Q&A

Numbers you must not fumble: **65%** · **9 runs** · **58–76%** · **15,442 → 5,323** · **946 → 1,480**

---

## 1 · Title — 15s

- KnowU is a personal AI that remembers more about you and sends less to the model.
- Built for Track 1, Cost of Intelligence.
- Lead with the number: **65% fewer input tokens, nine measured runs, every one a row in Snowflake.**
- Don't linger. The number is the hook; the proof comes on slide 5.

## 2 · The problem — 25s

- The normal way to personalize: append profile + history + conversation, resend it all on every request.
- Cost grows even when almost none of that context is relevant to what was just asked.
- From our own logs: memory grew from 5 facts to 8, and the full-context prompt went **1,173 → 2,566 tokens**. Nearly doubled.
- The flip: **the question decides what gets retrieved and how much is allowed to be sent.**

## 3 · How it works — 30s

- Three steps: Observe, Retrieve, Prove.
- **Observe** — apps, permitted titles, browser history, editor saves become work threads. Raw material never leaves the Mac.
- **Retrieve** — every question hits EverOS hybrid search *before* it hits a model. Approved memories come back ranked; a deterministic packer spends the leftover budget on evidence.
- **Prove** — one model call, then one aggregate row to Snowflake.
- Line to land: *"We retrieve a small prompt instead of compressing a big one."*

## 4 · EverOS — 30s

- This is the layer that makes the number possible.
- Built on the EverOS Personal Memory API v1 — and we use all of it:
  - `POST /memories` — approved facts and corrections written through
  - `POST /memories/flush` — force extraction so new facts are retrievable immediately
  - `POST /memories/search` — hybrid retrieval on **every single query**, not every session
- Why it matters: retrieval is cheaper than summarization and it's auditable — **no second model call.**
- Vendor-isolated `MemoryService`: the product never knows which memory backend it's talking to.
- The boundary is deliberate — approved memories go out, raw behaviour never does.

## 5 · The proof — 30s ⚠️ SLOW DOWN

- Nine real inference runs, logged locally, synced to Snowflake.
- **Average input-token reduction: 65%. Worst run 58%. Best 76%.**
- In absolute terms: **15,442 tokens of full context became 5,323** — about **1,124 saved per question**.
- Volunteer the caveat before they ask: the prompt we sent is the provider's actual reported usage; the baseline we didn't send is scaled by the same deterministic ratio and labelled an estimate on screen. **We never show an estimate as if it were measured.**
- Close: input tokens are the cost driver. 65% fewer input tokens is 65% off the input bill — any model, any scale.

## 6 · Why it compounds — 25s

- Same user, same app, two points in time.
- **5 memories:** baseline ~1,330, we sent 384 → saved **946**.
- **8 memories:** baseline ~2,487, we sent 1,007 → saved **1,480**.
- Savings per question grew **56%** as the assistant got more personal — the opposite of how personalization normally behaves.
- Why it's structural: full context grows with everything you've ever done; our prompt grows only with what *this question* needs. **The gap widens on its own.**

## 7 · Trust — 25s

- A cost claim nobody can inspect is a marketing claim.
- In-product, one question shows: which memories EverOS returned, which evidence fit the budget, Full vs KnowU tokens, and how they were counted.
- Be precise: the big prompt is **never sent**. It's an answer-equivalent baseline built only for comparison. The optimized prompt is the only thing that produces the answer.

## 8 · Snowflake — 25s

- Every answer writes one aggregate row through the Snowflake SQL API.
- The row carries baseline vs optimized tokens, tokens saved, reduction %, real provider usage, budget and units sent, cache counts, latency — and the measurement method.
- `CONTEXT_ECONOMICS_SUMMARY` rolls it into daily totals and average reduction: one saving becomes a trend you can chart, alert on, and charge back.
- What it never receives: the question, the answer, memory text, URLs, window titles, raw activity. Numeric only — the feature that would send prompt text ships **off**.
- Land: *"You can prove the economics without exporting anyone's life."*

## 9 · Live demo — 60s

1. **Ask with context** → *"What should I prioritize when building the production version?"*
2. Point at the retrieved EverOS memories and the selected-thread evidence that fit the budget.
3. Open **Context Economics** — read both numbers out loud, then the reduction %.
4. Show the Snowflake telemetry status or the prepared summary query.

- If the venue network is risky: play the recording and narrate the same four beats.
- **Pre-flight:** export `EVEROS_API_KEY` + `EVEROS_USER_ID`, hit **Seed demo memories**, confirm both indicators are green.

## 10 · Close — 20s

- KnowU makes the assistant more personal without making every request heavier.
- Raw behaviour stays local. EverOS holds the approved memory and answers a hybrid search on every question. Snowflake proves the economics.
- **65% fewer input tokens across every run we measured** — and because the baseline grows with the relationship while our prompt grows with the question, that gap widens the longer someone uses it.
- Closer: *"Every token should have to earn its place."*

---

## Q&A prep

**"Nine runs is a small sample."**
Agreed — it's demo scale. What matters is that the direction is consistent: no run came in under 58%, and the measurement is built into the schema so the same numbers keep accruing at any volume.

**"Is the baseline a straw man?"**
No — it's answer-equivalent. It contains the same retrieved memories and query-specific facts as the answer path, plus the profile, history, and conversation a conventional implementation would resend. We build it for real; we just don't send it.

**"How do you know the token counts?"**
The sent prompt is the provider's own reported usage. The unsent baseline is scaled by the same deterministic ratio and labelled an estimate. On Bedrock we use the native `CountTokens` preflight for exact counts.

**"Doesn't retrieval add cost and latency?"**
One search call, no extra LLM call. Summarization-based context compression needs a second model call — that's the cost we avoid.

**"What if EverOS is down?"**
Graceful fallback to safe derived local memories, and the run is labelled `local-fallback`. The user still gets an answer.

**"Why not just use prompt caching?"**
Complementary, and we do both — Bedrock cache points on eligible prefixes. Caching lowers the price of tokens you send; we send fewer tokens in the first place.

**"Where's the willingness to pay?"**
Different track — this is Cost of Intelligence. The economics slide is the product: cost per answer becomes a metric you can charge back.
