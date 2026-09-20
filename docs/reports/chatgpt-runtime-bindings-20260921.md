---
version_status: current
reviewed_at: 2026-09-21
---

# ChatGPT Sep 21 Runtime Compatibility

## Observed Failure

- APK 1794 (`79125c4b0`) reached the authenticated, empty, temporary group document.
- All five read-only send admissions returned `runtime_unavailable / base_context`.
- The compatibility sender then returned `draft_not_accepted`; no AI answer was delivered to the group.
- A read-only runtime-assets probe on the phone identified a new official rollout. The installed binding catalog ended at `web_20260915_b`. The failure is not evidence that ChatGPT lacks group analysis or that the VPN is still unavailable.
- The authorized request was left uncertain, not replayed. No private conversation, credential, or header was recorded.

## Reviewed Contract

The production loader now recognizes the exact `web_20260921` rollout:

| Role | Public asset | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-mhr2av0d0a9vqezt.js` | `b0152a193178594493616b57850993bcea832bb14a6714c11710e3e63d334dfe` |
| Shared | `4813494d-dbc08vb642hnn0un.js` | `fed049c50c3d83c8527f5cf7cd12bc3a0d54e62729928fb4d8ab96ee7d33cd27` |
| Conversation | `conversation-small-8auz4n865vyash62.js` | `ab034dc1e16d995b506483d083aa2eb82494abfcdecf85980029f41c4eedd780` |
| Composer | `8b34dbc2-lcl4lqvg2jgttftg.js` | `80cf749df5c971930f49fab28710fc75b4c95c21b6fd088277afc16e839809e2` |
| React | `2340486e-b3t78qt0psmq9d8c.js` | `091f0478804dd0611414cd745ed04eda6ebfd30f2a1bae8816d9c96f001bb2af` |

Public assets were downloaded without user credentials and parsed, not imported or executed. The existing AST comparison found 84 unique normalized candidates among 100 consumed exports. Ambiguous matches were resolved using actual dependencies, not the first candidate:

- Request status: the stop function imports the shared `vS` binding, not another same-shaped atom.
- Client conversation: `dX` uses the `WEB:` prefix, not the request-placeholder prefix.
- Project PIN: `yK` reads the account-key-scoped PIN getter.
- Account and library sessions: follow the current-account selector and the store implementing `hydrateSessionFromLibrary`.
- React API, DOM, root and Intl: original binding bodies remain identical; export names changed. Nested assignments sharing short identifiers are not top-level candidates.

Six changed contracts were reviewed separately: business context adds display metadata; model availability adds an optional denial description; share variant normalizes its result; history hydration adds navigation lifecycle acknowledgements while retaining `shouldApplyResponse` and pending-draft guards; attachment serialization adds provider metadata; attachment store retains the reset boundary. Temporary and tool owner components were located and pinned independently.

The fresh-send and recovery context accept the reviewed profile. Identity, active-owner, privacy, model, draft, one-shot authorization and no-replay checks remain intact. Unknown or mixed rollouts still fail closed. Canvas export/generation and writing-library profile gates were not widened by this group-send fix.

## Verification

- `test-chatgpt-runtime-sep21-public-evidence.cjs`: all 100 mappings, live bindings, exact hashes, public-source contracts, temporary/tool owners and unknown-revision rejection.
- Fresh text context, temporary dispatch and private text input: combined 95 tests passed, zero failed or skipped. The source test used both downloaded old and new assets, not only synthetic namespaces.
- Existing runtime compatibility and recovery admission/context: 60 additional tests passed, zero failed or skipped.
- Android Kotlin compilation and 20 targeted tests passed; XML confirms zero failures, errors or skips (preparation 7, session policy 4, diagnostics 4, adapter delivery contract 2, group feature 3).
- Release `v1.1.1795` (build 1795, source `6112eaca4`) succeeded. APK SHA-256: `13f4675593562d4e9fe642090e90dfb5b74752035ff6267136ce12424b4d626e`. Server artifact and manifest were verified; the registered Xiaomi was updated without clearing data and its installed build was read back as 1795. The Honor was offline.
- After the user unlocked the phone, a fresh two-message selection was submitted through the real group UI on 1795. The first two read-only admissions returned `context_unavailable`; attempt three returned `ready` approximately 3.9 seconds after executor start. This confirms the new runtime binding is usable on the phone, not only in fixtures.
- The subsequent one-shot send returned `official_runtime_v1:rejected` in approximately 25 ms. No AI answer reached the group. This request was not replayed. The generic receipt discards the lower-level rejection code, so the next change adds a single read-only `fresh_text_trial_state` inspection before destroying the failed isolated document, with a two-second deadline. It does not authorize, retry, or alter the write.
- The bounded failure inspection passed 15 targeted Android tests (inspection 4, diagnostics 4, preparation 7), with XML-confirmed zero failures, errors or skips. Tests cover stale and wrong-action receipts, cancellation, timeout, single completion and untrusted-payload rejection.

## Reproduction

Download the fixture's exact public assets from `https://chatgpt.com/cdn/assets/` to an untracked directory. Download the prior Sep 15-b fixture assets separately. Set `CHATGPT_PUBLIC_RUNTIME_DIR`, `CHATGPT_PRIOR_RUNTIME_DIR`, and `CHATGPT_AST_PARSER` to the reviewed sources and Acorn 8.15.0, then run the evidence test with `node --test`.

The analysis tool is `scripts/analyze-chatgpt-runtime-contracts.cjs`; the stable fixture is `scripts/fixtures/chatgpt-runtime-bindings-sep21.cjs`. Its output is advisory and must never automatically enable a new provider revision.
