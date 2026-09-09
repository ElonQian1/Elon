# September 10 runtime admission

Capability: `android_chatgpt_private_runtime_bindings_v1`, resolver version 8.
This repairs current-build compatibility for existing consumers; it does not
introduce an independent HTTP sender or complete every private capability.

## Observed regression

Normal APK 1623 returned legacy DOM model handles instead of private handles.
Its bounded 96-entry asset inventory omitted every runtime role and reported
truncation. Absence in that inventory was inconclusive, not an unsupported-build
diagnosis. The next diagnostic release, `1.1.1624` from `ebf545e02`, retained
role filenames within the same bound and exposed the new official build below.
It passed Release/lint/publishing and unattended replacement installation on
the trusted Xiaomi. No model selection or account preference write was made.

The passive model diagnostic initially remained `not_observed`. A fixture then
reproduced an independent lifecycle gap: adapter reinjection replaced the model
module's WeakMap although the composer retained its original model instance.
Model-state version 5 now retains that module across same-page reinjection.
It still invalidates evidence when the document changes. This is a diagnostic
fix, not permission to reuse stale account state.

## Public source evidence

These exact assets were observed on the phone and downloaded without credentials
from `https://chatgpt.com/cdn/assets/`. The anchor's actual imports identify all
three dependencies. The React module remains `2340486e-dyt4epctwx2pn2sj.js`.

| Role | File | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-jb0yl2d3l1jp8c93.js` | `7eaaebe4b4593c521a085e89e796323898c0d426012c69c6c341a120d8239ffb` |
| Shared | `4813494d-ilaxclpwvg5i0e40.js` | `3f28b7c766311f6a6a2ebf6986e872571459d694b58468fef5a68fe13f1787a4` |
| Conversation | `conversation-small-ng27r04netsz3e4o.js` | `00ae8fd8e7a9639909d773aca4aa79d86e8834a49307aaa3794a23690b5885dd` |
| Composer | `8b34dbc2-l0q54hwyus1gmzd1.js` | `22842341683a190c8b9c197e103a2d537faa88fc8d54b110efaeb8b199c9df57` |

Acorn parsed the prior admitted September 9b and current public modules without
executing website code. Declarations and lazy initializer assignments were
compared while preserving literals, operators, property names and object keys.
Fifty of 51 consumed exports had exactly one normalized structural match.
The independent `scripts/fixtures/chatgpt-runtime-bindings-sep10.js` records
all expected aliases. This comparison is supporting evidence, not automatic
admission of unknown future builds.

The async-state store had six identical candidates. Current official stop owner
`DN`, exported as `HHt`, reads `tt(c)` (actual shared import `BS`), request-active
getter `F` (import `$u`) and `lc.STREAMING` (enum import `het`). These actual
imports resolve legacy `Fx`/`Fl`/`v7`; the other store candidates are rejected.

The compiler-owned tool contract maps `H_n` to `Evn`; the temporary-chat owner
maps `DJt` to `nYt`. Both have unique prior/current structural matches. The
temporary owner retains the 30-slot memo contract and its exact current callback
was inspected. The application invokes the captured official closure only under
its existing context guards; it never reconstructs or executes that callback
from this report. The existing 265-slot tool contract remains unchanged.

## Input picker scope

The model trigger previously checked each selector in the input scope and then
globally before trying the next selector. An early page-header match could beat
a later valid composer picker. The adapter now tries scoped selectors, semantic
composer lookup and scoped candidates before the global compatibility lookup.
Three production-adapter tests cover those priorities and require private model
handles without a synthetic web-touch request. The existing plus-button and
attachment trigger policies are unchanged.

## Verification boundary

The 204-case focused run passed without skips, covering all aliases, wrong reused
aliases, document replacement, warm reuse, current owners, real stop/submit
consumer composition, model-module reinjection and composer trigger scope.
Evidence: `runtime-sep10-green-20260910-065017-680` in the repository Git log
directory. The adjacent consumer suite also passed; see
`runtime-sep10-consumers-20260910-065139-744`.

At this source checkpoint, the compatibility changes have not been installed or
accepted on the phone. APK 1624 proves the diagnostic observation, not the new
private model selection. Model/effort mutation, tools and temporary-chat owners
still require their own production acceptance. No latency, heat or power gain
is inferred from these fixture tests.

## Normal 1625 device result

Release `1.1.1625`, source `b73d79195`, passed the normal publisher, Release/lint
and unattended replacement installation on the Xiaomi. The local APK and remote
download agree on SHA-256
`c6109cb195daf1a142264968683e0b0efdb8c0d34facb7d057bff42044d45c7c`.
Build log: `runtime-sep10-release-20260910-065432-476` (432.8 seconds).

The authenticated production page now returns six `private_model_*` handles
with `model_runtime_context:ready`. This contrasts with the legacy handles
observed before the source fixes. The first production native popup displayed
the level slider and Advanced entry, and its Advanced page displayed version
choices. Read evidence: `model-admission-device-20260910-070157-536`;
native-popup evidence: `model-native-popup-1625-20260910-070231-780`.

The real native option was selected using Accessibility ACTION_CLICK: latest
version to `GPT-5.6 Sol`. A fresh private catalog marked that version selected
and the model summary changed to its versioned label. Selecting latest again
restored its selected flag. Android Back closed the popup and the production
model button reopened it. The final fresh root catalog confirmed the original
extra-high preset and a ready diagnostic, with the popup closed. No messages,
microphone use, new conversation, data clearing or independent proxy changes
were involved. Evidence: `model-native-version-select-encoded-1625-20260910-070700-304`
and `model-native-restored-1625-20260910-070954-909`.

Completed slice: current-build private model catalog, native popup/Advanced,
version selection with live readback and restoration. This is not verification
of server preference persistence, a subsequent send, every model, tier changes
or slider-drag submission. Those remain separate acceptance work. The Kotlin
diagnostic source compiled in Release, but its dedicated JVM test was not run
in this batch. Offline counts are 204 focused and 519 adjacent consumer cases.

The external UI harness initially expected the legacy model selector. The current
production port instead exposes `chatgpt-composer-option:model:<id>:<label>`.
Its first corrected attempt then failed before dispatch because a label with
spaces became multiple Android-shell words. The harness now accepts only bounded
model selectors and carries the selector as UTF-8 base64 across ADB. Three focused
tests and the actual native selection pass. A later reopen attempt was made while
the popup was still open; closing it first fixed that test precondition. These
harness failures are not private transaction failures and triggered no duplicate
model mutation. No third APK build was needed.
