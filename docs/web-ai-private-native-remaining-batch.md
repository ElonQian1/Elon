# Remaining private-native batch

Current implementation audit: 2026-09-05. This is a work list, not a declaration
that every private protocol has been reproduced. Reuse completed capabilities in
[the capability matrix](web-ai-private-transport-capability-matrix.md).

## Workflow

Implement coherent modules with targeted checks and separate commits. Do not
publish an APK for every small correction. Use one grouped install/acceptance
round after the candidate batch is ready. Existing proven native audio,
subtitles, dictation, read-aloud, directory cache, and mutations are not repeated
research. System alternatives remain explicit choices, not silent replacements
for website functionality. Persistent WebView identity remains intentional.

## Current batch

| Work | Code | Verification | Delivery |
|---|---|---|---|
| Private history to native wire format, citations, file/image descriptors | Implemented | JS and shared Android fixture passed | Await combined APK |
| Content-only refresh preserves current composer/voice state | Implemented | Targeted Android tests passed | Await combined APK |
| Bounded/coalesced image requests and no false empty-library success | Implemented | Targeted JS passed | Await combined APK |
| Private conversation attachment index, cache and native file sheet | Implemented | Shared JS/Android contract and targeted production tests passed | Await combined APK |

Root cause, exact modules, and check results are in
[the history contract](chatgpt-private-history-native-contract.md).
The attachment-index scope and acceptance are in
[the file index contract](chatgpt-private-conversation-files.md).
Five legacy source-location assertions also fail on the unchanged baseline;
their exact scope is recorded there. They are not a full-suite pass or a reason
to repeat already-verified private transports.

## Protocol gaps

| Area | Existing usable path | Actual remaining private work |
|---|---|---|
| Text send/regenerate | Native send ledger and official transaction; streaming observer | Fresh proof-bound private dispatch is not verified. Do not replay captured proof headers or declare official fallback a private POST success. |
| Model/effort/tools/temporary mode | Native presets/cache and official controls | Apply the chosen state through a confirmed private contract; cached menu labels alone are not proof of server selection. |
| Attachment upload | Native picker/progress; official upload owner | Observe and verify prepare/upload/finalize plus composer association before replacing the owner. Metadata display is not an uploader. |
| Images | Native gallery/previews/cache; official creation and library sync | Confirm private library pagination and generation transaction. Download queue improvements do not replace these endpoints. |
| Share/delete/conversation files | Native pin/rename/archive/move; private file-index candidate; advanced official options | File descriptors now use the private history GET. Download authorization, share/delete contracts, reconciliation and destructive confirmation remain. Do not substitute system sharing for official features. |
| Google direct send | Native cache and private response observer; official submit | Reproduce the current submit contract and transaction ownership; observed reply endpoints do not imply a working private sender. |

An unknown protocol remains a documented code gap. No guessed endpoint, fake
success, or automatic write replay should be added merely to make this table
look complete. Existing UI stays usable while each replacement is implemented.
