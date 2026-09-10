# Runtime Retry Store Observation

## Installed Baseline

Normal `1.1.1643 (1643)`, source `71e4845aa`, adapter 326/resolver 11, was
published and installed without data clearing on the trusted Xiaomi.
APK SHA-256: `b13be582dc7ba2a68c1b11afb73ebd1c375cd75a2b3c920cc287bfcf99a42e19`.
Release log `regenerate-runtime-owner-release-20260911-022123-619` passed in
473.9 seconds, including Release compilation/lint and remote artifact checks.

`native-retry-runtime-owner-1643-20260911-023003-705` passed the native row/button
and both private model catalogues (6/6 each). The actual native retry was invoked
once; the document stayed at generation 3 and the generation POST returned
200/stream. The receipt failed with
`official_runtime_v1:regenerate_unknown:timeout_stream_missing`, not the earlier
account mismatch. No replay/fallback write occurred. Original conversation and
stay-awake settings were restored. Overall retry acceptance remains failed.

The read-only follow-up `native-retry-1643-late-read-20260911-023724-383` passed in
38.8 seconds: the isolated fixture still had one user turn and a completed,
47-character marker-matching assistant reply with native retry rendered.
It restored the prior conversation/awake state. Because it revisited a fresh
document, this alone does not prove the failed request's new variant or latency.
No private content, account identifiers or credentials are recorded here.

## Evidence And Correction

Retained public `4813494d-fgzk5uadh2hlkw93.js`, SHA-256
`853f1aec75ae5d51c679e0bbf2e8b62c0b514f81a2f1f2cbd24b0e47ecba6b80`, exposes
the thread store `hS` as `GJ`. Existing thread getter `cS` (`jJ`, canonical `XM`)
reads that store via `gS = hS.getState`. `hS` is built by `_n`/`gn`/`pn`/`fn`;
the store factory returns `getState` and `subscribe`, with an unsubscribe closure
removing the listener. `gn` assigns those store methods to the bound function.
The middleware aliases `oFe`/`YPe` are identity and `hy`/`jke` wraps only writes.
These source links were inspected locally; no guessed endpoint or hook execution.

Resolver 12 adds this semantic export only to the evidenced profile. Retry
contract/runtime 10 subscribe only during the owned command and compare the
exposed store's thread with the existing getter. With no captured visible text,
the already-committed official reply can satisfy the unchanged new-variant and
original-parent checks. A final owner check after subscription prevents setup
reentrancy from changing accounts before invocation. No additional send, DOM
poll, token copying, voice modification or native transcript replacement.

## Verification Boundary

The finite baseline run `regenerate-store-observer-red-20260911-024015-855`
failed four of 26 cases: missing active subscription and no late store release.
`regenerate-store-observer-related-20260911-024401-759` passes 314 cases, including
29 focused store cases and all selected runtime-binding/model/retry guards.
No skipped/cancelled tests. These are offline fixtures, not live stream proof.

## Normal 1644 Acceptance

Source `0e137a23c73f18308fafa17f53a6ed6c0aff58dc` was published and installed as
normal `1.1.1644 (1644)`, adapter 327/resolver 12. APK: 40,117,633 bytes,
SHA-256 `c72111423e819df0e63925110a856bdbd3647c29adccbbc0da3ef442b00803e0`.
`regenerate-official-store-release-20260911-024713-688` passed in 404 seconds,
including compilation/lint, live version/artifact checks and replacement install.

`native-retry-official-store-1644-20260911-025412-059` passed in 62.7 seconds
including navigation and restoration. It confirmed both private model catalogues
(6/6), the rendered native row and enabled retry button, then clicked once.
The original synthetic prompt was reused; no additional user message was sent.
The command returned `official_runtime_v1:regenerate_observed`, native streaming
was observed, and changed marker-matching content finished. The native projected
message ID remained unchanged; the runtime receipt independently requires a new
official tree variant and the same original user parent. No second write or
fallback was issued. The original user turn, native surface, conversation and
stay-awake setting were preserved/restored; no Cookie or application-data clear.

`android_chatgpt_official_runtime_regeneration_v1` is `completed` only for this
ordinary authenticated text scope and remains default enabled. Do not repeat
its implementation or expand samples without a regression. This receipt does
not count frames by observer source or establish a latency/thermal improvement.
The passive SSE capture gap, first-word timing, project/temporary/image contexts
and independent Android HTTP remain separate unresolved scopes.
