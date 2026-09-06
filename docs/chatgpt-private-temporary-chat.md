# ChatGPT private temporary-chat state

## Status and scope

- Capability: `android_chatgpt_private_temporary_chat_state_v1`.
- Status: implemented, source-only candidate; not `completed` or device accepted.
- Contract version: 1. Production native temporary-chat selection invokes the
  inspected official runtime transaction and confirms its resulting state.
- This is a page-runtime private state bridge, not an independent HTTP privacy
  endpoint. WebView still owns identity, the router and official thread state.
- Ordinary empty/saved chat routes are covered. Project, custom-agent and work
  contexts, guest identity and unknown runtime schemas retain their existing path.
- Existing native official audio, captions, dictation and read-aloud are unchanged.
  Google remains last. No APK was built, published or installed for this batch.

## Inspected official source

The exact modules below were inspected on 2026-09-07. No endpoint, permission or
privacy mutation was inferred from a menu label.

| Official module | SHA-256 |
|---|---|
| `https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js` | `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5` |
| `https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js` | `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0` |
| `https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js` | `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1` |
| `https://chatgpt.com/cdn/assets/2340486e-dyt4epctwx2pn2sj.js` | `bd1f145733f12933c92dd18fbb8e982601c65ef22a41dd2f898fc8f357857261` |

The composer's `AKt` owns the temporary-chat button transaction. In an empty
conversation it clears attachment/personalization state, conditionally clears
personalization-dependent tools and updates the router search state with replace.
In an existing ordinary conversation it navigates to a **new** temporary chat.
An existing temporary conversation has no selectable callback; its button is a
read-only privacy indicator, not a way to make that conversation saved.

`AKt` is not exported. The adapter locates its committed button ancestor and
recognizes the exact inspected callback source. The compiler's `useMemoCache(30)`
is stored in `fiber.updateQueue.memoCache.data[0]`: slots 0/3/4 hold the client
thread ID, is-new and temporary inputs; 7/19 share the exact action; 20/21 hold
the read-only and temporary state. These slots are read, never mutated. Their
shape, identity and consistency are required before claiming this private path.
The stored action is never used to bypass the read-only button's missing callback.

The shared namespace exposes `XM` for an existing thread lookup,
`HM.getIsNewConversation` for current eligibility, `cX` for the official route-based
temporary signal, and `uo` for work-mode exclusion. Thread
`is_do_not_remember` and conversation `config.startDoNotRemember` are the privacy
inputs also consumed by official send preparation. A URL change alone is not
accepted as confirmation. The conversation module's `oD` is the inspected
navigation helper already captured by `AKt`, not a separately reconstructed call.

## Transaction and lifecycle

1. Bind one connected, committed button owner to its conversation, route,
   document token and private account identity. Reject ambiguous or stale owners.
2. Require the exact composer, shared and React runtime URLs to have been loaded
   or module-preloaded by the official page. Lazily import the shared namespace
   once per document, single-flight, with a 1.5-second timeout and 10-second
   failure cooldown. Warm operations do not reopen an official menu.
3. Compare the closure's captured is-new/temporary inputs with live thread and
   route state before any action. A just-sent message may update the thread
   before React commits a new button. Wait at most 1.5 seconds for a fresh
   committed callback in the same conversation; otherwise fail without mutation
   or a DOM retry. Never execute the stale empty-chat branch on a saved chat.
4. Invoke the existing official callback once. Keep its attachment cleanup and
   new-chat navigation semantics. Do not synthesize touch/click events, call
   React hooks/render functions, or alter existing conversation privacy directly.
5. Confirm the desired route signal and effective thread privacy together. An
   empty-chat toggle must remain in the same client conversation; a saved-chat
   transition must produce a new empty conversation at the home route.
6. Observe for at most 2.4 seconds after dispatch. Join duplicate same-intent
   requests without another mutation; reject opposite intent while pending.
   Exceptions after invocation still require readback, never automatic DOM replay.
7. Cancel on account, document or unexpected conversation/route changes. Keep
   unconfirmed post-write state non-settable until actual confirmation arrives.
   No recurring timer remains once the operation settles.
8. Unknown schema/module before writing may retain the existing control path.
   A known but temporarily uninitialized thread is not an unsupported feature.
   Account/runtime objects and credentials remain inside the page closure.

## Production UI and validation

`WebChatProductionHeaderActionPolicy` preserves an observed read-only temporary
state instead of discarding it and showing an inactive preset. The action sheet
shows it as selected and disabled. Pending confirmation is disabled too.
`WebChatTemporaryChatIntentQueue` can confirm a read-only target state, but cannot
issue a mutation while selection permission is absent.

- `chatgpt_web_private_temporary_chat.js`: guarded transaction and observation.
- `chatgpt_web_adapter_temporary_chat.js`: existing production adapter integration.
- Layout supplies the source node; asset/bootstrap changes only register lifecycle.
- `scripts/test-chatgpt-web-private-temporary-chat.js`: synthetic runtime,
  cleanup, stale compiler state, read-only, timeout and production wiring checks.
- Header action and intent queue JUnit tests cover the native read-only state.

The focused Node run passed 97 cases across this bridge, the existing temporary
adapter, model state and attachment-composer bundle checks. This is fixture-based
verification, not a live official transaction. Native JUnit tests were added but
not run in this source batch; Android compilation remains part of grouped testing.

Grouped device acceptance still needs empty-chat on/off, saved-chat to new
temporary chat, read-only existing temporary state, attachment/tool cleanup and
a message sent immediately before a toggle. Confirm history placement with an
explicit synthetic send and check unknown-version behavior preserves existing
functionality. No latency, heat or power improvement is claimed before measurement.
