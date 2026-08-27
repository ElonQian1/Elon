---
capability_id: android_google_web_conversation_url_lifecycle_v1
implementation_status: completed
verification_status: device_verified
production_default: true
---

# Google Web conversation URL lifecycle

Google AI Mode uses different URL shapes for navigation and durable conversation identity.
They must not share one persistence rule.

## URL classes

| Class | Example shape | Allowed behavior |
|---|---|---|
| AI Mode home | `/aimode` | Open a blank Google AI Mode surface |
| Prompt execution | `/search?...&q=...&udm=50` without `csuir` | Submit or observe the current prompt only |
| Durable conversation | `/search?...&csuir=<opaque id>` | Directory, body cache, history open, restart resume, and official fallback |

`q` is user input carried by a search execution URL. Loading that URL again can execute the
prompt again. It is therefore never a conversation identifier, even when the page has already
rendered an answer.

`csuir` is treated as an opaque provider conversation identity. The APK does not decode,
invent, export, or modify it except for the already verified official directory mapping.

## Persistence rules

- `sanitizeNavigableUrl` allowlists bounded official Google AI Mode pages. It may accept a
  prompt execution URL because the official send flow needs to navigate there.
- `sanitizeConversationUrl` additionally requires `/search` and a non-empty `csuir`.
- The conversation directory, selected path, visited-body snapshot, and `last_ai_mode_url`
  preference accept only `sanitizeConversationUrl` output.
- Starting a new conversation clears the previous durable resume pointer before the official
  page starts the new flow.
- Startup with a legacy prompt execution URL opens `/aimode` and removes the invalid pointer.
- Recovery while the current page is a prompt execution URL opens `/aimode`; it never reloads
  the `q` URL and never falls back to an unrelated older conversation.
- Existing AtomicFile records without `csuir` are ignored during bounded decode. Cookies,
  account state, and app data are not cleared.

The native message snapshot can remain visible while the official page settles, but transient
page state cannot create a durable history row. A history row appears only after Google exposes
a stable conversation identity through the official page or verified directory response.

## Fallback and privacy

The official WebView remains authoritative. This policy does not add a private send request,
replay POST data, inspect credentials, or persist conversation text in diagnostics. When stable
identity is unavailable, the safe fallback is a blank AI Mode surface plus the existing native
draft/retry state, not re-execution of an old prompt.

## Verification

Release unit tests cover legacy cache migration, startup, recovery reload, history navigation,
official fallback, and stable snapshot matching. Same-version research APK `v1.1.1317 (1327)`
then completed two distinct exact-marker turns with both streaming phases observed. A separate
force-stop/relaunch case restored the same stable conversation with two messages and one assistant
reply, remained non-streaming for twelve read-only checks, and produced no reply without a new
send. Both cases restored the original conversation and did not clear cookies or app data.
