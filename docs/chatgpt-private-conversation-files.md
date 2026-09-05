# Private conversation attachment index

## Scope

Capability: `android_chatgpt_private_conversation_files_v1`.
Code is published and installed in `v1.1.1540`, not yet device UI verified.
Candidate adapter: `262`; history transport: `18`; directory requests module: `2`.
This is an attachment **index**, not a private uploader or file downloader.

The production conversation action sheet now exposes `Conversation attachments`
through `web-chat-conversation-action-files`. It reads the selected conversation
using the existing authenticated same-origin history GET. No menu discovery,
conversation navigation, composer replacement, microphone action, or write is
required to list files. Concurrent history/index reads share one HTTP request.

## Data and state

- The projection reuses existing history attachment/image descriptors and walks
  the selected branch, including messages older than the 80-message display
  window. Hidden/internal and unselected regeneration records stay excluded.
- Output is bounded to 100 descriptors and 4096 traversed nodes. A known partial
  result is labeled partial. Unknown, malformed, ambiguous, or failed history
  is not published as a successful empty list.
- Native session memory retains eight conversation indexes, keyed by canonical
  conversation identity. Project and canonical paths share one entry. Fresh
  entries display immediately for 60 seconds; stale entries remain visible
  while a targeted refresh runs. This cache is not persisted across APK restart.
- Results must match the newest pending request for the exact conversation.
  Expired requests and old-document callbacks cannot overwrite current state.
  Clearing account history clears indexes and invalidates pending file reads.
- Dismissing the sheet stops native receipt polling. Explicit cancellation or
  provider changes invalidate pending UI handoffs; no sheet or file-detail
  dialog should reopen from a stale callback. There is no idle polling loop.
- Requests reuse the established identity/timeout/health policy. Failure keeps
  cached descriptors with an explicit retry. No automatic write replay or
  automatic official-page navigation is added.

Only descriptors enter the native index: message ID, file name, role, kind and
MIME type. Private file handles, signed download URLs and credentials remain in
the page. Selecting an item shows metadata and offers its original conversation;
it does not pretend to download a file. Opening the already-current conversation
is a no-op, preserving its draft and voice state. Navigating to another one uses
the existing draft guard and tracked navigation.

## Verification

The shared fixture `private-conversation-files-contract.json` is consumed by both
the JavaScript producer test and the real Android bridge parser test.

- JavaScript directory tests: 14 passed, including branch isolation, old
  attachments, mixed media, limits, failed/unknown responses, GET coalescing,
  retry after cooldown, and already-injected module upgrade.
- Existing private-history projection: 12 passed; private transport regression
  suite passed.
- Android release-source compile and first shared-state/parser regression batch:
  88 tests passed. Runtime inventory follow-up: 6 tests passed. Final production
  attachment, cache, receipt, lifecycle and catalog batch: 24 tests passed.
- Grouped APK build, release and data-preserving installation passed on `v1.1.1540`.
  Device UI acceptance remains pending; see [delivery evidence](web-ai-private-native-remaining-batch.md#grouped-release).

Two broader legacy source-contract classes ran 26 tests with five failures.
Their exact failing predicates are also false on the untouched baseline
`f9956fe67439be2bb2b7a19cf670521015833650`: model sheet location, composer-menu
dispatch whitespace/order, refresh-session constructor location, project-scope
wiring location and refresh-coordinator constructor location. These checks still
expect earlier module locations; they are not counted as passing and are not
silently weakened. Follow-up should migrate these source contracts to the actual
owners with behavioral coverage. The directory-cancel assertion was moved to its
new domain dispatcher as part of this batch. No full-suite pass is claimed.

Device acceptance should open files from another conversation while keeping an
unsent draft, reopen from cache, fail one refresh without losing cached rows,
then dismiss during a read and verify no late dialog. Use synthetic attachments.
No device latency, battery or temperature improvement is claimed from offline
tests. Upload prepare/finalize, download authorization, generated-image library
pagination, share and delete remain separate protocol gaps.
