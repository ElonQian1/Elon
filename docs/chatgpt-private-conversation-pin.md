# ChatGPT private conversation pin

## Product behavior

The production conversation menu exposes `置顶` or `取消置顶` from the cached nullable pin
state. An unknown state is never displayed as pinned. Selecting the action does not navigate
the visible chat, discard a draft, or wait for a menu DOM node.

## Transaction boundary

The persistent ChatGPT identity WebView owns the same-origin request and all authorization
material. It sends one `PATCH /backend-api/conversation/{id}` request with the desired
`is_starred` Boolean. Android receives only a request receipt and a bounded result code.
Cookies, authorization values, request headers, conversation text, and account data never
cross this bridge.

An HTTP success updates the in-memory conversation directory and a read-only
`/backend-api/pins` request reconciles the result. A transport timeout is an unknown outcome:
the write is not replayed, and bounded delayed pin reads continue until the official state is
confirmed or the reconciliation window expires. A recently reconciled pin state temporarily
wins over a stale conversation-directory response so eventual consistency cannot flip the UI
back. A contradictory pin read immediately after an HTTP success is treated as lagging index
data, not as a rollback. Authentication failure or HTTP rejection leaves the Android cache
unchanged.

## Reliability rules

- Exactly one mutation can be active in a page document.
- A write has a bounded timeout and is never automatically replayed.
- An uncertain write gets bounded, read-only delayed reconciliation before failure is reported.
- Failures enter a short cooldown; repeated or authentication failures open a longer circuit.
- The production UI reports success only from a completed correlated command receipt.
- Failure starts a read-only directory refresh and offers the official conversation options.
- The official page remains authoritative and can be used for manual repair.

Capability ID: `android_chatgpt_private_conversation_pin_v1`.

## Verification

Completed on the production friend-chat surface with APK `v1.1.1504`, adapter `243`. A single
write pinned the active conversation, the native directory retained that state across a refresh,
and a second single write restored the original unpinned state. The earlier timeout-late-success
case was also observed without a write replay and drove the delayed reconciliation policy.
