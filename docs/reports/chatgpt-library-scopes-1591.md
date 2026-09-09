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
