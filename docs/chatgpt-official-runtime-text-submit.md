# Official runtime text submission

Capability: `android_chatgpt_official_runtime_text_submit_v1`.
Status: implemented source candidate, not live-accepted or completed. It is an
official page-runtime bridge, **not an independent Android HTTP/private POST
transport**. Include it in the grouped ChatGPT APK; do not reimplement it while
waiting for production acceptance.

## Scope and ownership

The native send transaction can invoke the current official `submitComposer`
with `text_action` and `requireDispatchAcceptance`. This removes the ordinary
fill-editor, wait-for-send-button and click sequence when the recognized runtime
is already ready. Fresh proof preparation, model/tool policy, conversation state
and generation remain owned by the website. Existing stream observation and the
native send ledger are reused; voice, dictation and Google are unchanged.

The bridge requires the inspected module to be present, one committed composer
context, one file store, a matching conversation/controller, and known readiness
and policy fields. It binds the document, identity, route and selected leaf before
invocation. Credentials remain page-local and never appear in receipts. Unknown
contexts, structured input and pending/ready attachments leave the existing
sender available **before any runtime write**. The text-only candidate admits
ordinary conversations and the currently selected new project/temporary context;
other routes are not guessed.

After invocation, there is exactly one writer. Synchronous `accepted: false` is
an unsent rejection. Acceptance requires the official completion to resolve to
`true` while ownership still matches. Errors, ambiguous completion and a 15-second
observation timeout are indeterminate, never a reason to send again by another
route. The bridge retains its in-flight ownership after timeout until the
underlying completion settles. An invocation exception or malformed receipt
retains ownership until document replacement; there is no automatic reload or
write replay. Recovery for such a protocol mismatch remains an acceptance item.

`text_action` does not reset the official editor. Only an unchanged, nonempty
draft matching the submitted text is cleared after acceptance; subsequent user
edits survive. The existing native receipt policy records `OFFICIAL_PAGE`, not
`SAME_ORIGIN_PRIVATE`, including indeterminate receipts. A successful receipt
does not prove that the response finished or that first-token latency improved.

## Source evidence

Inspected public assets, retrieved without account credentials:

- [Composer runtime](https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js),
  SHA-256 `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`.
- [Conversation runtime](https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js),
  SHA-256 `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.

In the first asset, `j$n`'s `xu` gates eligibility and returns
`{accepted, completion}`; `Su` is passed as `submitComposer` into `nOn`. `UDn`
writes those props into the shared context store. Existing official consumers
call this exact `text_action` transaction with `requireDispatchAcceptance`.
`TOn`/`EOn` assembles the live generation state and awaits dispatch acceptance;
only the current-draft branch resets the editor. The second asset defines the
shared store's `getSharedProps` and subscription contract. These are public
source observations, not a captured successful authenticated request.

## Verification and next acceptance

On 2026-09-07 the focused Node command passed **60 tests**, with zero failures or
skips, across `test-chatgpt-web-text-runtime-submit.js`,
`test-chatgpt-web-private-text-transaction.js`,
`test-chatgpt-web-text-transaction-lifecycle.js`,
`test-chatgpt-web-send-settle.js`, and
`test-chatgpt-web-private-send-observer.js`. It covers current React ownership,
context/draft changes, attachment exclusion, new conversation IDs, uncertainty,
single dispatch, production orchestration and asset-bundle syntax.

New native receipt tests are written but await grouped Android compilation and
execution. No APK was built, published or installed for this module. Browser
navigation timed out, so this round obtained no live runtime/send evidence.

In the grouped production UI round, confirm that the actual committed context
resolves, send one synthetic text, retain the `official_runtime_v1` receipt and
stream result, and check draft preservation and conversation ownership. Check
the admitted project/temporary contexts without claiming unsupported scopes.
Independent fresh-proof private HTTP dispatch, private regeneration, measured
latency and resource improvement are still separate unfinished work.
