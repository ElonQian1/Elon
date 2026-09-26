# Win Group Image Runtime Repair

Status: implementation and offline checks passed; installed acceptance pending.
Scope: the existing group selection, native attachment byte bridge, ChatGPT
temporary conversation and group reply flow. No proxy or credential changes.

## Findings

The prior reviewed website profile no longer matched the current deployment.
Its Rspack sender also explicitly rejected native attachments. Independently,
the root-route context reader rejected a local temporary thread as soon as
it acquired a server conversation ID, hiding an otherwise valid answer.

## Reviewed Public Contract

Public CDN assets were downloaded without account headers, parsed with Acorn,
and never executed offline. The manifest contains 6,606 parsed modules with
zero parse gaps. The additional profile is `web_20260926_rspack`.

| Asset | Relevant modules | SHA-256 |
| --- | --- | --- |
| `238022.59b5f57fb1.js` | `OS` auth, `c3` scope | `6f7bd36448de3f7eae80df630905a1c91aabd87e5944a1ee3abad4262c9e64b0` |
| `376616.59ccddcc11.js` | `sA` identity atoms | `af79538b61da798a511c7bc0b0829bae08f7c91b9d5e687d8280983a6eab481a` |
| `36750.72bb8d082e.js` | `wg` conversation, `q4q` ready attachment projection | `c16732fed697a4f7702099c145b383ce444728b46dd9cde606b85b690424401c` |
| `498514.937d7074b7.js` | `vG.N` upload, `vG.E` remove, `vG.f` uploads | `a974252a77066cb34fb3988a700dab24672364717cd60a603ceca52c1e7a4165` |
| `934244.c57697fda7.js` | `CUv.a` send transaction | `357d351b2fe03598cadc7efdd796acf697698b221e381e8f1d32bb5a6123cb96` |

Admission requires the exact observed runtime `633146.6ed5d111e4.js`, entry
`908190.d446cd6dfd.js` and auth asset, plus executed module cache exports.
Unknown deployments remain unknown; no guessed alias or `require(id)` call.

Read-only MCP attached to the live personal Win WebView during this repair
observed another deployment, `manifest-4da31bb4.js`, with 6,698 parsed modules
and zero parse gaps. It is admitted separately as `web_20260926b_rspack`:
runtime `633146.e8647fddbe.js`, entry `908190.80f53e7a66.js`, auth/scope
`238022.33322e145f.js`, identity `376616.5a8098a4e7.js`, conversation `JqV`
and attachments `q4q` in `184143.0e420b28d4.js`, composer `vG` in
`109686.cd293bd41c.js`, submit `CUv` in `934244.b068731984.js`.
The corresponding account/branch/upload/send contracts were rechecked;
`wg` now means an unrelated module, so alias substitution is not safe.
The inspection tool now emits module-level hashes as well as asset hashes.

## Implementation

- APK and Win share the Rspack adapter. Native byte leases remain unchanged.
- Reviewed `vG.N` performs official quota checks, conversion, reservation,
  upload and processing. A complete batch of scoped ready entries is required.
- `q4q.m` supplies the official file specifications to `CUv.a` through
  `additionalAttachments`; `preserveDraft` protects the existing editor.
- Captured file entries are removed only after a confirmed send ACK. Failed
  or ambiguous sends do not silently retry or send a text-only question.
- Upload cancellation removes only captured upload IDs. A timed-out operation
  remains single-flight until it settles, including late admission callbacks.
- The current committed local owner can acquire a valid server UUID without
  losing native rich-message projection on the unchanged temporary URL.

## Verification

- 58 targeted Rspack tests passed, including the server-ID transition,
  multimodal send, partial batch, timeout, account changes and ACK cleanup.
- 180 existing upload, Win assembly and group-reply tests passed.
- A stale test pinned adapter version 207; it now checks the actual minimum
  attachment-observer contract instead of rejecting every later adapter.
- Real upload, image understanding, native answer readback, group publication
  and APK device verification must be recorded after installing the candidate.

This report does not mark the capability completed before live acceptance.
Tools/projects on the new Rspack sender remain separately scoped contracts;
this repair must not imply that all website features have been migrated.
