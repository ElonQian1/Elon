---
capability_id: android_chatgpt_fresh_project_regeneration_v1
implementation_status: implemented
verification_status: offline_and_android_build_verified_native_retry_pending
delivery_status: normal_1765_published_installed
production_default: false
scope: existing_owned_global_project_plain_text_retry
---

# Existing Project Retry

## Gap and Evidence

Ordinary independent retry is already accepted on 1726/default on 1728. Existing
project Send is accepted on 1753/default on 1754. Those successes do not prove
project retry: both regeneration capture and request creation explicitly excluded
all projects. The new project test reproduced `scope_unsupported` at `base_project`
before any preparation or write.

The second Sep 15 public sources were parsed without execution. Their pinned
hashes and semantic assertions are in
`scripts/test-chatgpt-project-regeneration-public-evidence.cjs`. `emr` retains the
original user parent and selected/tree conversation mode in a Variant request.
`GB` prepares Next with that user parent, empty messages and the same project
mode; the shared serializer retains mode and variant purpose and omits messages.
No new endpoint, copied request or replayed credential is introduced.

## Implementation

Regeneration context v8 reuses the existing plain-project admission in text
context v26: same account, exact project/route, loaded GLOBAL context, no shared
continuation, restricted project, business agent, temporary mode, attachment or
tool retry. The original question identity and current answer remain required.

Request v9 removes the blanket project rejection, retaining project serialization
and all other variant checks. Transaction v37 uses its existing ledger, fresh
prepare/proof, private stream, native projection, Stop and verified history.
History must match the project and user as well as the exact observed variant.
Drafts are not cleared. Uncertain post-dispatch results cannot fall back or resend.

Adapter 428 is included in grouped normal Release 1765. This scope is enabled only by
`__elonChatGptFreshRegenerationProjectsEnabled === true` or the existing one-command
fresh trial. The read-only admission probe recognizes the armed trial without
consuming it. Project Send's existing default cannot enable project retry, and
the trial never changes defaults. Ordinary retry remains enabled as before.

The existing native Retry button is reused; there is no new UI or writer. Initial
eligibility still uses the committed official model/retry menu and conversation
owner. This change replaces project retry dispatch, not all DOM discovery or the
WebView identity layer. No audio/subtitle/dictation or proxy code changed.

## Verification and Remaining Work

- `project-retry-batch-20260915-170036-788`: 627 Node tests passed, zero failures,
  skips or cancellations, five seconds. New cases cover canonical/project routes,
  single-command opt-in, prepare/variant bodies, one user turn, draft retention,
  owned stream/history/branch selection, project/account drift, foreign history,
  Stop and no duplicate POST. Existing private-send/input/recovery tests passed.
- `project-retry-public-final-20260915-165857-163`: one current hash-pinned public
  source test passed, zero skips, 11.5 seconds. This proves the reviewed client
  protocol shape, not server acceptance.
- The integration receipt can be `accepted` once the owned stream arrives while
  history remains pending. Tests separately assert pending state and refuse
  foreign-project history; receipt acceptance alone is not completion evidence.
- Android compilation, grouped publication and replacement installation passed
  on 1765; production native ready/idle was confirmed. See the
  [grouped release receipt](chatgpt-runtime-bindings-20260915.md).
  Native project Retry acceptance remains pending. Use one harmless existing
  owned-project question, arm one trial,
  verify a single new assistant variant/no new user message and preserved draft,
  then restore the prior view. Promote only this scope after that real pass.
- New-project, attachment/tool/feedback/temporary/restricted/shared-project retry
  and fully DOM-independent initial retry discovery are not covered here.

Do not redo the implementation because native acceptance is pending. Reuse the
existing trial, receipt, Stop and history tools in installed 1765 or a successor.

## Installed Fixture Admission Check

On Sep 15 after voice was confirmed closed, a read-only native probe reused the
previously resolved project-send fixture on installed 1765 / adapter 428. Both
native and backing paths matched the recorded project conversation; both showed
33 messages and 12 user turns with an empty draft. Six current user turns did
not match the controlled synthetic fixture. The fixture guard correctly refused
write eligibility instead of widening its prompt allowlist to cover real use.

`project-retry-native-admission-1765-20260915-173925-762` stopped at fixture
ownership. The bounded diagnostic
`project-retry-native-fixture-diagnosis-1765-20260915-174153-338` identified the
mixed user turns on both projections. Neither run armed a trial, sent a message,
regenerated an answer, changed project membership or cleared data. Both restored
the original native conversation. No conversation content was exported.

This is an acceptance-fixture failure, not a rejected private regeneration
request: private admission was never reached. Do not repeat this fixture or
count it as a provider incompatibility. Next use a newly isolated owned-project
synthetic question, keeping the existing original-user/variant/history/native
UI checks. No new Android build is needed merely to perform that acceptance.
