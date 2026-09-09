# Library navigation and temporary attachment checkpoint

Date: 2026-09-09. Production APK: normal `1.1.1591`, adapter 306.
Source: `b609e779b07f8ce318ca575d19fd6022c8e43dc4`.
APK SHA-256: `f43d1d0d16b99f00cc45ba06a923bcabd2207b3b0469ae5414ca17ee6bec6aa1`.

## Completed scope

`android_chatgpt_private_library_browser_v1`: native More, folder and back
interaction is **completed** for this ordinary authenticated library. Existing
read/cache implementation is reused; no production catalogue rewrite was needed.

- External semantic UI activation exercised the production native library.
- More retained all 21 earlier rows and extended the list to 41 unique handles.
- A real folder returned four rows with its own handle and one breadcrumb.
- Back restored the same 41-row root and cleared the breadcrumb.
- Original blank chat/draft and temporary awake lease were restored.
- No file mutation, upload, microphone or account-state change was performed.
- More took 3,895 ms including the external UI runner; this is not a render or
  network-only latency benchmark.

Evidence: `library-navigation-1591-20260909-070954-241` in the shared Git
`ai-command-logs` directory. The final harness additionally includes awake-lease
restoration in its aggregate pass condition. Do not repeat this completed scope
without a current regression. The earlier 1589 next-page failure's cause remains
unattributed; this normal-release pass closes the current navigation check.

## Temporary attachment failure

Evidence: `temporary-attachment-1591-scope-20260909-071705-973`.

- One synthetic 78-byte text file uploaded through the private transport:
  `private_attachment_associated`.
- A single user turn produced the requested marker and the actual file's first
  line. The uploaded content was available to the assistant.
- Send acknowledgement was `official_runtime_v1:unknown:context_changed`.
- The post-send temporary-state check failed, so full scope acceptance remains
  **pending**, including the subsequent personal-library inventory comparison.
- The fixture was not replayed. Local fixture and blank normal chat were restored.
- The temporary-mode toggle receipt does not attribute runtime versus compatibility;
  this case does not independently prove the private toggle transport.

An earlier harness attempt used the official uploaded-file count instead of the
native staged-fixture count and stopped before sending. Its leftover local
fixture was explicitly removed and zero pending counts verified. That attempt
is not an APK upload failure. An additional route-only probe was inconclusive:
the public snapshot strips URL query parameters and the probe did not await the
temporary indicator before sending. It is not temporary-chat acceptance evidence.

## Scoped source correction

The private submit owner previously admitted a server-ID-bearing homepage only
for a confirmed guest. Temporary chats can also remain at the homepage after
receiving a server ID. Temporary-state capture had the same mismatch.

Submit v16 now requires the exact temporary homepage plus the live official
thread's temporary privacy, selected state, non-new state and absence of work or
project context. Temporary-state v4 admits the same persisted home only with
privacy and existing-thread proof. Account, document, committed component,
conversation, attachment store and controller ownership remain checked. No
uncertain write is replayed and no URL-only privacy assumption is introduced.

Public September 9 source examined through the existing versioned bindings:

- Composer `8b34dbc2-cj4kfo18e1ldvw16.js`, SHA-256
  `c24245fc260253db68ae531c586174a3eeff95e694b7c8a0e76eb77eae5fe906`.
- Shared `4813494d-bgyv5408fxme7xxv.js`, SHA-256
  `72ed87dd6d8a5241abac73d9c720f8e92bf87bbee7980331fcf042fea64d14ca`.
- The existing temporary button becomes read-only after the first turn. Its
  label alone does not encode selected state. Existing shared thread getters,
  not a new endpoint or export guess, supply the privacy proof.

Regression evidence: two new cases failed before the correction (71/73 passed);
the final related attachment, temporary-state, submit and September 9 binding
batch passed **221/221**. Both acceptance scripts pass PowerShell parsing.
Log: `temporary-home-regression-final-20260909-073204-334`.

## Normal 1592 retest

The candidate was published and installed as normal `1.1.1592`, source
`b01005610e7516d40548e55ad10928865a7ce58f`, APK SHA-256
`5ceb100bb0d36880651515d36b3a6bbae0fd545960186ccdf47396b79af50e28`.
Release log: `publish-temporary-home-fix-20260909-073609-333` (458.3 seconds).

`temporary-attachment-1592-20260909-074508-965` reproduced the failure: private
upload, actual file-content reply and exactly one user row passed, but the send
receipt still reported `context_changed`. The post-send control existed with
`selected=false` and a ready homepage composer. The blank normal chat, local
fixture and awake lease were restored. No uncertain request was replayed.

The first source gap and unit regression were **not sufficient to resolve the
live failure**. Do not claim this temporary scope completed or infer retention.
Project attachment and other unverified scopes are not completed. Proven audio, subtitles,
dictation, read-aloud and authenticated stop/follow-up are unchanged. Google stays
after the remaining ChatGPT acceptance gate.

## Actual runtime correction

Same-version local research APK and a separate synthetic text turn established
the real state transitions (`temporary-runtime-trace-1592-20260909-075905-493`):

- Temporary query and official selected signal stayed true, with no extra query.
- Server ID changed from absent to UUID; official is-new changed to false.
- The current temporary button's committed memo agreed and became read-only.
- Legacy `thread.is_do_not_remember` remained false before and after the turn.
- The composer shared-provider wrapper changed. Conversation, controller, file
  store, editor and document were unchanged. No account or user content was logged.

Submit v17 permits this post-acknowledgement provider replacement only while
the real owners still match. Pre-dispatch capture continues to reject a replaced
provider. Temporary v5 confirms the exact official router signal against the
committed control's memo, rather than incorrectly equating it with legacy thread
metadata. The same cached private control supplies the submit ownership proof;
there is no second DOM scan or independent temporary-state implementation.

The delayed-render fixtures now model actual control commit separately from
legacy metadata. Related tests passed 229/229, including detached controls,
foreign conversations/controllers/file stores, pre-send replacement and no replay.
Log: `temporary-live-shape-final-20260909-080512-192`.

An attempted module-only hot replacement retained the old layout's captured
adapter reference and is not valid whole-candidate acceptance. The local research
APK was replaced with hash-verified normal 1592; blank chat/draft restored and
the private debug forward removed. Full normal-candidate delivery/acceptance
remains the next gate.

## Normal 1593 retest

Normal `1.1.1593` was published from `58242e23a` and replacement-installed on
the trusted Xiaomi. APK SHA-256:
`61db5b443ba20728a64a790a64bfa48653bc255dc820f5a1a7ec49194a02abd0`.
Release log: `publish-temporary-live-owner-20260909-081153-295` (440 seconds;
Gradle succeeded in 6m32s). Final related tests: 229/229,
`temporary-live-owner-final-20260909-081019-095`.

`temporary-attachment-normal-1593-20260909-081942-402` (63.8 seconds) confirmed:

- Private upload: `private_attachment_associated`; attachment phase completed.
- Exactly one user row and a reply containing the actual synthetic file line.
- Private send: **`official_runtime_v1:accepted`**. The previous false
  `context_changed` send acknowledgement is resolved for this scope.
- Post-reply temporary control still reported `selected=false`, although the
  control existed and the homepage composer was ready. Aggregate acceptance
  failed with `temporary_mode_not_confirmed`; do not mark the scope completed.
- The personal-library after-count was not reached. Neither non-persistence nor
  backend retention is claimed. The toggle transport remains unattributed.
- Blank normal chat, local fixture and awake lease were restored; no uncertain
  send was replayed. No microphone, Cookie or application-data reset was used.

The remaining issue is post-reply temporary-state projection, not upload or send
acceptance. Next diagnosis must compare the current committed read-only control,
its private observer and the native manifest; do not weaken ownership guards or
infer privacy from the label. Do not repeat the completed ordinary library scope.
The publication manifest and device both independently confirmed 1593 afterward;
the native chat was no longer foreground at that final read and was not reopened.
Only the MCP forward remained, with the temporary CDP forward removed.

## Read-only indicator root cause

`temporary-indicator-live-trace-1593-20260909-084410-650` reproduced the state
projection failure with one synthetic text turn in a same-version local research
build. Before and after the turn the official route signal, exact temporary
homepage and committed 30-slot control memo all agreed on selected mode. The
control changed from mutable/new to read-only/existing as expected.

Before the turn, six ancestor callbacks included two references to the exact
privacy transaction. Afterward, the transaction references disappeared but four
tooltip/presentation callbacks remained. v5 incorrectly required a recognized
privacy action whenever any callback existed, even on the read-only indicator.
It rejected this valid owner; the wrapper fell back to the generic label and
reported `selected=false, stateSettable=true` in the native header manifest.
The send was accepted and the two-row test chat was restored.

Temporary state v6 separates these contracts: mutable controls still require
exactly one recognized action identity matching the committed memo. Read-only
controls require no privacy action, while allowing unrelated presentation
handlers that are never invoked. Live route, account, document, conversation and
committed memo validation are unchanged. A retained privacy action on a read-only
control is rejected as contradictory rather than exposed as a stale toggle.

Both new regression cases failed before the correction; all 231 related cases
passed afterward (`temporary-readonly-regression-after-20260909-084548-971`).
The local research package was replaced with hash-verified normal 1593, the CDP
forward removed, and authenticated/ready empty native chat verified. Full normal
candidate acceptance is still required before closing temporary attachment scope.

## Normal 1594 failure and complete owner capture

Normal `1.1.1594` from `073c6f585` was published and installed; APK SHA-256
`15502c5b5133e7d181f7b15a3cb9f0ccb2544787b3b60d78d4d025a2ab943a3e`.
`temporary-attachment-normal-1594-20260909-085817-460` again confirmed private
upload, one user row, actual file reply and `official_runtime_v1:accepted`, but
failed post-reply selection. v6 fixed the read-only callback condition, not every
earlier capture condition. No aggregate pass or persistence claim was made.

The same-version research build exposed both earlier mismatches on a blank page:
the official client ID has the form `WEB:<uuid>`, and the committed owner has
memo-cache lengths `[30, 2]`, not just `[30]`. Account, document, observed asset
profile, owner commitment, exact action identity and conversation identity were
all valid. v6 rejected the ID before reaching the cache/callback checks. The
second cache belongs to a nested hook; the owner's first 30 slots still match
the inspected schema exactly.

Temporary v7 accepts the observed prefixed UUID and the additional two-slot
cache while retaining all first-cache, action, ownership, route and read-only
checks. Unknown namespaces, malformed IDs, extra or unknown cache shapes remain
rejected. The new positive cases failed before the change; 236 related tests
passed afterward (`temporary-owner-shape-after-20260909-091826-008`).

`temporary-candidate-state-proof-20260909-092338-943` loaded only an isolated
candidate observer into the research page. It correctly reported selected/mutable
before the synthetic turn and selected/read-only after it, while the unchanged
production wrapper still reported the old false state. A separate candidate
transaction confirmed on/off through the exact official callback without DOM
clicking and restored the normal blank route. These prove candidate capture and
transaction behavior, not final native UI acceptance. WebView rejected CDP live
editing (`setScriptSource functionality no longer available`); no official source
was changed. The research build was replaced with hash-verified normal 1594 and
the temporary CDP forward removed before delivery.

The native attachment acceptance now waits for both selected and read-only state
after the reply. It must pass on the normal release before this scope is closed.

## Normal 1595 acceptance completed

Normal `1.1.1595` from `eb0a0d1d9` was published and replacement-installed once on
the trusted Xiaomi. APK SHA-256:
`c96fb21b83872ae5b3c65fd374b83b61171e7a0b339014afac785b0f74531ada`.
`publish-temporary-owner-capture-20260909-092822-365` passed in 496.8 seconds;
Gradle passed in 7m36s. The release manifest and unattended device readback both
confirmed build 1595. No research package or CDP forward remains.

`temporary-attachment-normal-1595-20260909-093734-262` passed in 34.4 seconds
through production native input/send and the existing MCP handlers:

- One plain-text test attachment: private upload associated, phase completed.
- Exactly one user row, with the actual synthetic file line in the reply.
- Private send acknowledged: `official_runtime_v1:accepted`.
- Post-reply native manifest: `temporary_selected=true`, `temporary_readonly=true`.
- Personal-library fixture count remained 1 before and after. This is the bounded
  inventory check, not evidence of server-side retention policy or deletion.
- Normal blank chat, native/official drafts, local fixture and awake lease restored.
- Toggle receipt remains unattributed; no inference of its route from empty detail.

Capability `android_chatgpt_private_temporary_attachment_upload_v1`, scope
`new_temporary_single_plain_text_attachment`, is **completed/device_verified**.
Reuse without another acceptance run unless a current regression is reported.
The post-reply selected/read-only indicator is also device verified. This does
not complete existing temporary, image/PDF, project or mounted-source variants.
Those remain on the scoped work list; Google remains after the ChatGPT gate.
