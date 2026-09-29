---
version_status: current
reviewed_at: 2026-09-29
implementation_status: verification_in_progress
---

# September 29 ChatGPT Runtime

## Evidence

The WeChat-record group acceptance reached one protected image, then stopped
before question dispatch with `private_upload_rspack_build_unreviewed`.
Read-only Win MCP research observed `manifest-91fabf9a.js`, entry
`908190.8e6a26feba.js` and runtime `633146.681744a17f.js`.
The manifest's 157 public assets were downloaded without account headers and
parsed using `research-chatgpt-rspack.cjs`: 5,804 modules, zero parse gaps.
Raw sources remain in local ignored evidence, not the repository.

| Role | Public asset / module | Reviewed contract |
| --- | --- | --- |
| Auth | `238022.7e88f56120.js` / `OS` | Auth generation and workspace/account switch guards |
| Scope | `434385.a7f2495691.js` / `c3` | Executed `AppScope` singleton |
| Identity | `908766.3f419afb2f.js` / `sA` | Account `d`, user `i`, admission `j` |
| Conversation | `184143.2270b2c39e.js` / `ar` | Server ID `i`, current node `z`, origin `w`, project `K`, status `T`, mapping `G` |
| Composer | `869553.f7b4b2b625.js` / `vG8` | Prompt/model/uploads atoms unchanged; upload `P`, remove `F`, model compatibility `y` |
| Attachments | `184143.2270b2c39e.js` / `q4q` | Ready-entry projection `m` retains file ID, dimensions and MIME |
| Submit | `934244.436d1e7992.js` / `CUv` | `a(scope, options)`, preserved draft, additional attachments, dispatch acceptance and request ownership |

Module SHA-256: composer `03977d2e3c39d63378dcc1bc0363efe498e0b6e0b14d14b4d96199e19a4831f8`;
conversation `5948fcd33840f19187408aa451ae18d4f033235096ec09d67718c32002407916`;
submit `4d2f802953807c0f015832ef2a11df040ff7dd3e374a8b5c12503a0ffef9eaee`.

## Implementation

A separate `web_20260929_rspack` profile maps the reviewed operations to the
existing adapter contract. Official exports are never mutated or executed by
`require(id)`; old profiles remain intact. Unknown builds and missing methods
remain unavailable, not assumed compatible. The reviewed upload still owns
quota, conversion, reservation, bytes, processing and scoped ready entries.

98 focused runtime, upload, message, Win assembly and byte-bridge tests passed,
including changed aliases, cancellation, hydrated-model validation, ACK-only
cleanup, and rejection before writes when any required method is missing.

## Acceptance

Native release and live image/group delivery are pending for this profile.
The original failed task was not dispatched and generated no group answer.
Build/test success is not a substitute for actual delivery and source readback.
