---
capability_id: android_chatgpt_fresh_new_conversation_text_dispatch_v1
implementation_status: partial
verification_status: failed
production_default: false
scope: authenticated_new_personal_or_owned_project_text
---

# Fresh New-Conversation Dispatch

September 13 extension of the [accepted independent sender](chatgpt-fresh-text-dispatch.md).
It reuses its request ledger, security-aware page HTTP, stream decoder, native
projection, stop and history recovery. It does not add another sender or editor.
New-conversation scope requires an explicit trial until grouped native acceptance.
The accepted existing-conversation default is unchanged.

## Native Acceptance Failure On 1706

`fresh-new-native-ui-20260913-183220-505` used the actual native composer and
Send button on the installed release 1.1.1706, with the one-command trial armed.
There was no runtime seed: exactly one new-conversation candidate was sent.
The private request was dispatched and accepted; 31 owned stream events arrived,
including input-message, handoff and stream-complete events. History then stayed
at `parent_mismatch`, with `pending=true` and no confirmed reconciliation. The
native reply/follow-up/restoration case therefore **failed**, not completed.

The concrete gap is first-send parent ownership in authoritative history and
subsequent runtime-tree reconciliation. Current checks accept a direct
`client-created-root` parent or its reviewed legacy empty-root equivalent. The
actual rejected parent shape has not yet been inspected. Do not assume hidden
system ancestors, accept arbitrary UUID parents, or loosen branch checks merely
to retire the writer. Existing-conversation success does not prove this scope.

The pending controlled fixture is retained locally at
`$GIT_COMMON_DIR/ai-acceptance-fixtures/fresh-text-pending.json`; it contains no
credentials and is not committed or printed. Use read-only resolution first,
not another send. The original route was not preserved by this failed run and
must not be guessed. No conversation navigation, draft clearing, replay,
WebView reload or APK replacement followed the failed write. The awake lease
was restored. Production 1706 has no app WebView debugging socket, so direct
CDP inspection was unavailable; its debugging/login policy was not changed.

The canonical smoke now has `-NewConversation`, rejects outstanding handoffs,
and requires private send receipts, owned stream events, history reconciliation,
native/server route agreement and unchanged prior user identities. An ambiguous
click/result blocks cleanup and saves a local handoff. Default new-conversation
scope stays disabled. The earlier `origin_not_idle` attempt sent nothing; it was
a harness error corrected to use main draft-presence plus the provider draft.

Focused script tests cover 15 negative send proofs, six identity cases and
nine cleanup cases; the existing retry/trial contract suite also passes.
This acceptance-tooling batch changes no Android source and builds no new APK.

## Evidence

Only the retained, hash-pinned `web_20260912` public sources are parsed; downloaded
bundles are not executed. `test-chatgpt-text-dispatch-public-evidence.cjs` records
the shared/conversation/composer hashes and asserts these contracts:

- New conversations have a registered `WEB:` client owner and the exact
  `client-created-root` parent. The request omits `conversation_id`; it must not
  invent a server ID or substitute an arbitrary root UUID.
- Shared `IP/H1e` binds the server ID to the same conversation object, moves the
  registry entry and updates the client/server mapping atomically. `gY/QJ`
  identify and resolve the provisional owner. No manual store replacement.
- Composer `A1t` uses `Jsn/DDn` for the first requested default model and its
  remembered selection. Dispatch carries `requested_default_model` and
  `one_off_model_override` when present; preparation does not invent that field.
- `qHt` navigates through the official router, including canonical project
  routing. Its navigation argument can be a guarded callback (`r_n`); `HK`
  supplies the entry key needed to reject a changed navigation owner.
- The history callback receives raw data before `oQe` normalizes a legacy empty
  root to `client-created-root`. Only that exact root exception is supported.

Binding v23 exposes these helpers on this reviewed profile only. Request v4,
context v5, owned stream v2 and transaction v12 consume them. Existing runtime
send, projects, tools and their acceptance boundaries remain separate.

## Ownership And Recovery

Admission requires a normal, non-temporary personal workspace; an empty,
registered new conversation; its current model/effort/tool; no attachments or
competing writes; and an unchanged document/account/navigation entry. Project
roots reuse the [existing project scope and authorized headers](chatgpt-fresh-project-text-dispatch.md).
Unknown runtime profiles, shared/business contexts and configured continuations
are not silently treated as ordinary first sends.

The private request owns its decoded HTTP/topic stream. A matching input message,
an assistant message or a reviewed resume control may bind the returned server
ID once. Other metadata, another registered owner, another user message or a
later different conversation ID cannot authorize adoption. The existing SSE/v1
decoder publishes text to the native session after identity binding.

The stop request reads the acquired server ID, not the original null snapshot.
Before the ID arrives, it cannot consume the one-shot stop conduit. Authoritative
history must match the submitted user/parent/branch, project and privacy state.
Only then may official hydration and navigation complete the transaction.

Delayed project navigation callbacks expire on cancellation, timeout, changed
identity or route; a late result cannot pull the user back. Rejected navigation
does not retire an otherwise reconciled writer. Recovery reads that same turn,
never sends it again. A response lost before any server ID leaves an uncertain
writer; it does not search unrelated conversations or guess an ID.

Initial ownership still uses the committed composer host. Subsequent ownership
checks use memory, not send-button availability. This is not composer-free cold
startup, Android HTTP, WebView removal, or a measured thermal improvement.

## Offline Evidence And Next Acceptance

- `fresh-new-full-regression-20260913-124605-020`: 270 tests passed, zero failures
  or skips, including pinned source contracts, binding compatibility and shared
  stream regressions. The real module composition covers first send plus a
  second existing-conversation send, stop/partial text, project association,
  single ID adoption, SSE/v1 and history-before-navigation.
- Failure cases cover missing server ID, wrong branch/root/privacy/identity,
  changed navigation entry, rejected/late navigation and duplicate native
  commands. Successfully applied history with failed navigation stays pending;
  no second POST or automatic send fallback occurs.
- The first navigation-failure test asked for recovery before the asynchronous
  stream had finished. It was corrected to await history entry; that fixture
  failure is not recorded as product or live acceptance.
- This source batch changes JavaScript only. It has not built or installed an
  APK, sent an account message or measured phone behavior.

Use `fresh_text_trial_start` for one current document/account/route command,
or explicit `__elonChatGptFreshTextNewConversationsEnabled === true`. Projects
and tools additionally require their existing admissions. No production flag
is silently changed by source tests.

Resolve the recorded parent mismatch before repeating native acceptance. From the actual
production native UI, start one controlled new ordinary conversation, send,
verify one server conversation and complete native reply, then follow up in the
same conversation. Accept a project root separately and restore prior state.
Temporary, attachments, tool combinations, no-assistant stop and real lost-ID
network recovery are not implied by either ordinary sample.
