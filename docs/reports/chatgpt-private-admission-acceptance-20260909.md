# Private admission acceptance, 2026-09-09

Base: `1e023dace4f06a593973ba1dc38ec7f5fa5b0731`.
This is a narrow regression batch, not completion of the private-native Goal.

## Installed build acceptance

Normal Release `1.1.1603`, adapter 306, was downloaded and its SHA-256 verified:
`8aea0d102eb47c6559caf977f33b92636174742f0484de0c1c0b1a04f81c5503`.
Size: 40,031,749 bytes. An initial install raced the incomplete download and
failed APK parsing; the verified completed file then installed successfully
with `adb install -r`. No application data or WebView identity was cleared.

The production `social_ai` chat was opened through native MCP controls, selecting
ChatGPT. Its current project home had a current authenticated adapter but no
ready composer. No message, microphone action, upload or account write was sent.

| Case | Actual observation | Result |
| --- | --- | --- |
| Cached conversation directory | About 440 ms; 140 source conversations and 5 projects, marked stale | Passed without composer |
| Private file library | `mcp_6` succeeded in 1,543 ms; `library_ready`, 21 items, not stale or partial | Passed without composer |
| Global directory forced refresh | `mcp_2` failed after 27 ms: official sidebar entry not found | Failed; dispatch was not counted as provider success |
| Native sidebar | `open_chat_side_menu` accepted; native sidebar-open state true | Semantic control passed; no screenshot-based visual claim |

The user subsequently changed the foreground to another application. MCP health
became unavailable; the sidebar-close request could not be confirmed. No forced
foreground change, app restart or repeated bootstrap was performed. Conversation
navigation and drafts were not changed during the read-only cases.

## Follow-up implementation

- `chatgpt_web_adapter_conversation_directory_requests.js` previously routed
  unscoped refresh directly to the DOM sidebar. Only project-scoped requests
  attempted a private read. The unscoped route now uses the existing directory
  parser and a dedicated bounded same-origin refresh owner.
- The refresh owner uses the already-supported first-page conversation endpoint
  and shared identity acquisition. It coalesces duplicate requests, supports
  cancellation, and rejects responses after document/account/owner replacement.
  It preserves cached older/project rows and does not claim full pagination.
- Existing pin/archive/rename/project-move transactions now belong to account
  mutation admission, not composer admission. Confirmation and exact target
  validation remain. The private mutation owner validates context around every
  await and never replays PATCH after an uncertain outcome or context change.
- Explicit private-disable still preserves the official directory path. A
  temporary identity/read failure does not become a DOM-capability error.
- Native recovery/failure copy distinguishes unavailable identity and changed
  context from a provider rejection; it does not claim an uncertain write failed.

Existing stable capabilities are reused. The added first-page refresh scope is
`android_chatgpt_private_directory_first_page_refresh_v1`, implemented with
offline checks; **not device-verified or completed**. The earlier completed
passive directory/project-read capability is not re-labelled as unimplemented.

## Verification boundary

Node regression runners cover context replacement, no write replay, cooldown
ownership, stale/late results, bounded read lifetime, private directory dispatch,
cancellation, cold identity acquisition and production asset wiring. The existing
directory test now reads asset order from its current owner, `ChatGptWebAdapterAssets`,
instead of its previous location in `ChatGptWebPageAdapter`.

Follow-up code is not part of installed 1603. Targeted Android verification and
Git delivery are recorded below. New candidate behavior needs
the next grouped APK acceptance; the 1603 passes above do not validate it.

Remaining: full private directory pagination/project-catalog refresh, the actual
project-page recovery cause, project attachment acceptance and the other scopes
in the remaining-batch matrix. Do not start Google before the ChatGPT gate passes.

## Offline delivery evidence

- Pushed modules: `884cd452a` (account mutation context/admission), `dc2b17abc`
  (private first-page directory refresh). Shared test fixture extraction is
  `16093e3b7`; no second implementation of the mutation protocol was introduced.
- `private-admission-node-complete-20260909`: 43 Node runner cases passed,
  including the existing mutation/directory assertion suites.
- `private-admission-android-20260909`: Release source/test compilation and
  56 targeted JVM tests across six suites passed; 266.6 seconds. No APK was
  assembled or published by this check.
- Source-size and document-modularity guards passed. The remaining-batch index
  has its existing approaching-size-limit warning; the detailed evidence lives
  in this focused report rather than further expanding that index.
- The 1603 installation and safe production read acceptance remain separate
  from this source candidate. Follow-up publication/device verification is
  deferred to the next grouped APK, matching the batch workflow.
