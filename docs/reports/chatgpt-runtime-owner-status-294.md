---
version_status: evidence
reviewed_at: 2026-09-07
---

# Runtime editor ownership and progress extraction

Scope: follow-up to the actual production APK 1547 text-send regression.
Adapter 294; runtime submit 7; text orchestrator 6. Source/test candidate,
not a new completed capability or an independent native HTTP sender.

## Observed boundary

- APK 1547 returned the exact synthetic answer, but its receipt reported
  `private_fallback:template_unavailable`. The first runtime admission result
  was discarded, so this did not identify the actual failed gate.
- Native messages also contained a trailing four-character progress bubble
  after the correct answer, while streaming was false.
- This continuation had no connected ADB device. Browser navigation timed out.
  No microphone, account reset, private conversation deletion or new live send
  was attempted. The previously deleted synthetic fixture must not be replayed.

## Public source evidence

The current public [composer module](https://chatgpt.com/cdn/assets/8b34dbc2-nhot65scqrg20d6p.js)
was inspected locally; its SHA-256 was rechecked as
`36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733`.
This is source evidence, not an authenticated runtime capture.

- `KZn` obtains the ProseMirror editor with `XL(e)` and calls
  `y.current.appendChild(i.dom)`, assigning the prompt id and textbox role to
  that imperatively mounted node. The editor node itself need not own a React
  fiber; the nearest React host can be its parent. The old resolver only
  inspected the editor node and reproducibly rejected this structure.
- `M1n` exposes `isComposerSubmissionReady: rc` and `submitComposer: Su` through
  `ikn` / `GOn`. Its `yu` eligibility check requires `rc` even for `text_action`.
  Removing readiness checks would therefore not repair this contract.
- Official progress is rendered under `data-streaming-response-fallback` and
  `data-streaming-response-indicator`; the dot indicator also has an explicit
  marker. Whole-turn fallback serialization previously treated this markup as
  answer text. This is a reproducible extraction defect consistent with the
  observed bubble, not proof that every blank/status symptom has this cause.

## Changes

1. Resolve at most 12 connected ancestor hosts, stopping at the first React
   host and refusing an uncommitted host even if a higher ancestor is committed.
   Do not cross body/document root or search sibling editors. Keep unique
   shared/file stores, document/account/route/controller identity and readiness
   checks. Reparenting before dispatch invalidates the captured owner.
2. Report fixed pre-dispatch reasons such as `react_owner_unavailable`,
   `shared_store_unavailable`, `file_store_unavailable`, `submission_not_ready`
   and `conversation_route_mismatch`. DOM fallback retains the runtime reason
   alongside the existing private-relay reason, without another polling loop,
   prompt, credential, object dump or second post-invocation writer.
3. Exclude explicit progress/accessibility markup from answer text and media
   observation. Keep ordinary prose even if it literally says "Thinking";
   keep real voice commentary with `role=status`. Preserve raw DOM indices for
   action/stream matching and do not move streaming status to an earlier answer.

## Verification

Focused Node regression: **117 passed, zero failed/cancelled/skipped**.
Suites: text runtime, runtime fallback diagnostics, composite answer extraction,
runtime attachments, stop wiring, regeneration wiring, send settling,
streaming policy, private-send observer and image assets. Production asset
assembly is syntax-checked by the runtime suite.

Coverage includes imperative editor mounting, detached/uncommitted/deep/root
boundaries, reparenting, mismatch diagnostics, unchanged ownership/readiness,
single dispatch and uncertain-write exclusion, prepared attachment cleanup,
status-only and mixed-content turns, nested progress, indicator canvas,
literal status wording and voice captions. The old attachment reinjection test
now compares the installed runtime version to the module's exported version,
not a stale hardcoded version 5.

No APK build/publish/install was run for this small source batch. Continue the
grouped workflow. No latency, heat or battery improvement is claimed yet.

## Next device acceptance

Use the actual production native UI and one new disposable synthetic text
transaction. Require an `official_runtime_v1:accepted` receipt, one user turn,
one completed answer and no trailing progress bubble. If admission fails, use
the retained exact runtime gate; do not repeat share/delete/download acceptance
or label DOM fallback as private-runtime success. Preserve the original draft
and selected conversation and restore them afterward. If this narrow case
passes, move to still-unaccepted prepared attachment/context cases, Google last.
