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
must not be guessed. At the end of the 1706 failure batch, no conversation
navigation, draft clearing, replay, WebView reload or APK replacement had
followed the failed write. The awake lease was restored. Production 1706 has no
app WebView debugging socket, so direct
CDP inspection was unavailable; its debugging/login policy was not changed.

The canonical smoke now has `-NewConversation`, rejects outstanding handoffs,
and requires private send receipts, owned stream events, history reconciliation,
native/server route agreement and unchanged prior user identities. An ambiguous
click/result blocks cleanup and saves a local handoff. Default new-conversation
scope stays disabled. The earlier `origin_not_idle` attempt sent nothing; it was
a harness error corrected to use main draft-presence plus the provider draft.

Focused script tests cover 15 negative send proofs, six identity cases and
nine cleanup cases; the existing retry/trial contract suite also passes.
That acceptance-tooling batch changed no Android source and built no new APK.

### Read-Only Parent Diagnostics

The follow-up adapter 377 adds `chatgpt_private_protocol_probe` mode
`history_parent`. It reads only the current ordinary conversation through the
reviewed runtime history loader with `shouldApplyResponse=false`. No composer
capture, send, replay, navigation or manual tree update is performed by the probe.
The five-second, single-flight read is bound to document, route, runtime profile
and account; owner changes discard the result. Existing network observers remain
unchanged when the command surface upgrades.

Only an allowlisted description of the latest user turn's ancestor chain crosses
the bridge: roles, ID categories (not IDs), structural equality/child-link checks,
hidden flags and bounded counts. Node and Android tests cover rejected scopes,
cycles, broken/missing links, late callbacks, timeout and private-field rejection.
This is diagnostic infrastructure, **not** evidence that an unexpected parent is
valid. New first-send admission and reconciliation stay unchanged pending the
exact failed turn's parent sample.

The diagnostic was published and nondestructively installed as `1.1.1707 / 1707`,
source `1c102939b0e6d9021151c46ed542ff0f70611099`, APK SHA-256
`9462278fb42c3997554516da4be53616df6a834eb174244bad640a1c999da6a8`.
`history-parent-release-20260913-192436-603` completed build, publication,
remote manifest/hash verification and Xiaomi installation in 472.3 seconds.
`history-parent-node-20260913-191727-956` passed 36 tests; the focused release
JVM run `history-parent-jvm-20260913-191729-314` passed 19 tests. Neither suite
had failures or skips. A prior Node run only required updating an existing
probe-version assertion to 26; that fixture mismatch was not a device failure.

The first current-history read, `current-history-parent-probe-20260913-193341-368`,
returned a bounded timeout. The warm read
`current-history-parent-probe-warm-20260913-193435-837` returned `observed` and
the allowlisted structural chain. It included hidden system ancestors and an
unclassified root with an empty parent. This was a **different conversation**,
not the pending first-send fixture; it does not authorize relaxing root or
branch ownership checks and is not a first-send acceptance pass.

Read-only navigation after the 1707 upgrade inspected three current-day cached
candidates; none matched both the pending user-message ID and exact controlled
prompt. An initially rejected third navigation succeeded after the previous
navigation settled; it was not proof of a deleted conversation. The later
provider directory status reported stale data and `directory_timeout`, despite
the native cache remaining usable. Therefore the bounded cache miss does not
prove that the accepted request was absent from the server. Its pending receipt
is retained, no message was resent, and the original route remains unknown.

The separate native first-page lookup did not execute: its external semantic
runner rejected `foreground_mismatch` while WeChat was foreground. The runner
diagnostic initially omitted `ComparisonFailure`, hiding that specific reason;
`fresh-native-directory-runner-detail-20260913-195552-074` identified it. This
is an acceptance-tool failure, not evidence that the first-page private read
failed. The awake setting was restored and no write or replay was attempted.
The pending fixture lacks the accepted server conversation ID, so rebuilding
the page-local writer cannot recover that ID. A future acceptance handoff must
preserve exact accepted ownership locally before replacing a pending page;
do not guess a conversation from title or activity date.

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
