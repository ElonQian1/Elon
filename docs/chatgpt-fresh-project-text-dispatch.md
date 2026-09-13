---
capability_id: android_chatgpt_fresh_project_text_dispatch_v1
implementation_status: implemented
verification_status: offline_verified
production_default: false
scope: authenticated_existing_personal_owned_project_text
---

# Fresh Project Text Dispatch

September 13 source extension of [the accepted plain-text sender](chatgpt-fresh-text-dispatch.md).
It reuses its queue, native stream, owned stop and history recovery. It does not
create another editor or transport. Project admission remains explicit until one
grouped production-native acceptance succeeds; ordinary sends keep their accepted
default. [Project Writing Blocks save](chatgpt-writing-blocks-project-save.md) is
a separate implemented batch, not evidence that project generation is accepted.

## Observed Contract

The retained `web_20260912` public sources are parsed, never executed. Exact
bundle hashes and AST checks are in `test-chatgpt-text-dispatch-public-evidence.cjs`.

- Official `FB -> AB` uses the same `/f/conversation/prepare` endpoint and carries
  the selected conversation mode. A project uses `kind=gizmo_interaction` and
  its `gizmo_id`, not `primary_assistant`. The shared mode serializer is identity;
  `CJ/SJ` drop the display-only `gizmo` object before submission. Project
  instructions are not copied into the prompt or invented request fields.
- `JO` derives possible business-agent context from the current turns, project
  and conversation. This scope requires an explicitly absent result; business
  agents are not silently submitted as ordinary projects.
- Official `OB` obtains the current account's authorized locked-chat PIN and
  locked-project ID through `aK` and `Yr(h2())`. `iK` supplies
  `x-openai-locked-chats-pin` only for the matching project. Binding v22 exposes
  these reviewed helpers only on this profile. Headers remain page-local,
  immutable per command, and absent from diagnostics/native receipts.
- Request v3 preserves project mode through prepare, security metadata and final
  POST. It supplies the observed project header to preparation and streaming;
  the owned stop contract remains unchanged because the reviewed stop endpoint
  uses only its own conduit and trace headers.

## Scope And Safety

Context v4 supports `/g/g-p-<32hex>[-slug]/c/<uuid>` and a plain `/c/<uuid>`
route owning the same loaded project. It verifies route/server ID, unique
registered conversation, personal workspace, project ID/mode, loaded history,
non-temporary/non-shared state, selected parent/model and existing restrictions.
Unknown mode fields, custom GPTs, business agents, active connectors, attachments,
continuations, configured handoffs and competing writers remain outside scope.

The project and authorized header join the pre-dispatch fingerprint. Moving a
conversation, switching identity, changing the parent/model/tool or revoking the
captured header stops dispatch. In-flight ownership and reconciliation recheck
project scope; authoritative history must contain the same `gizmo_id`, own user
message and parent/branch. No later response may turn a project into a personal
chat or overwrite another project's tree. A result uncertain after POST is
recovered through reads, never a second send.

Initial ownership still uses the committed composer host. Later checks read
owned memory, not DOM readiness or the website submit callback. This does not
claim composer-free startup, Android HTTP, WebView removal or lower temperature.

## Admission And Acceptance

Transaction v11 reuses `fresh_text_trial_start`: one command, current document /
account / route, at most 120 seconds. The private-transaction switch still
applies. `__elonChatGptFreshTextProjectsEnabled === true` permits this source scope
explicitly; it is not set by the production default. Search/Picture additionally
require [their existing tool admission](chatgpt-fresh-tool-text-dispatch.md).
Pre-dispatch failure may claim the accepted sender once; after POST it cannot.

Grouped acceptance should use one controlled existing owned project with an
assistant parent. Arm one trial and click the actual native Send button. Require
one user turn, a real reply, fresh-owner stream delivery, terminal reconciliation
and unchanged project association. Restore the previous route, draft and tool.
Promote only the accepted scope; a mock send or installation is not acceptance.
Locked-project authorization and project/tool combinations require their own
owned sample before claiming those variants production-verified.

## Verification

- Source commits: `00866f4f1` (reviewed bindings), `2965b33df` (request/history)
  and `3480494cb` (owned admission and integration). All are pushed to main.
- `fresh-project-final-regression-20260913-115103-024`: 242 tests passed, zero
  failures/skips. Covers context, request, transaction, stream, stop, recovery,
  wiring and runtime-binding compatibility. The composition test runs the real
  context/request/transaction/history modules against synthetic HTTP and proves
  exactly one prepare, POST, history apply and stream finish.
- `fresh-project-public-evidence-20260913-114601-731`: 12 public-source checks
  passed, zero skips; confirms mode, business context, account-scoped project
  headers and the unchanged stop contract.
- The first composition fixture omitted the production private-transaction
  switch and used an overlong synthetic command ID. Both were correctly refused
  before HTTP. The fixture was corrected; failed runs are not acceptance.
- This JavaScript-only batch does not change Kotlin or package another APK.
  No device or user conversation was operated. Group it with the pending tools
  and project writing-save batch for Android build and native acceptance.
