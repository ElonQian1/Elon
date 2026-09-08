# ChatGPT runtime release 1554 evidence

Date: 2026-09-08. Publication, workflow acceptance and private-path acceptance
are separate results.

## Delivery

- Reader commit `0ec8daae6`, production wiring commit `9dbd46f02`.
- APK `1.1.1554`, code `1554`, adapter `299`, generation reader `1`.
- APK SHA-256:
  `503b5a860334bf045a95ca16ddb5ddb4f96fb098d5ef8c56fe7c7303130d7f85`.
- Release Kotlin/Java compilation and assemble passed in 6m44s; publisher
  completed in 450.7s. Remote publication and pinned Xiaomi `install -r`
  completed, with installed build 1554 verified.
- Packaged reader, streaming policy and adapter JS matched source after newline
  normalization. No Cookie, application data or login was cleared.
- 129 focused Node runner cases passed. The previous policy failed the new
  stale-marker/current-completion regression. The production entry test also
  proves the whole stream object reaches the reader. Source/document/ownership
  gates passed. Existing voice workflows were not retested or changed.

Logs: `runtime-generation-integration-20260908-20260908-081017-558` and
`runtime-generation-release-20260908-20260908-081839-741`.
Optional LAN firewall and broad cleanup warnings did not invalidate delivery.
Mandatory task finalization remains separate from the publisher's cleanup.

## Production samples

The unlocked Xiaomi used the production social AI input/send handlers via APK
MCP, with adapter 299, guest account context, idle voice and empty drafts.
Before any write, guards rejected non-fixture conversation content or active
generation. No screenshot, official-page input, microphone or repeated write
was used. The second send ran only after the first fully settled.

| Sample | Exact reply first observed | Native settled | Matching user / assistant |
| --- | --- | --- | --- |
| D | 4,334 ms | 6,473 ms | 1 / 1 |
| E, same open conversation | 2,220 ms | 4,308 ms | 1 / 1 |

Times are measured from the native send action and include two-second polling
resolution. They are not precise network first-token or WebRTC latency. Both
samples ended with native generating false, unlike the stuck 1552 sample.
No claim is made that the new reader branch itself fired: no branch receipt was
exposed, and current guest ownership resolution can still reject the reader.

Neither sample accepted independent private POST or official runtime submit:

- D: `private_fallback:template_unavailable`,
  `runtime_fallback:submission_not_ready`.
- E: `private_fallback:capture_dynamic_proof`,
  `runtime_fallback:conversation_route_mismatch`.

Device log: `runtime-generation-1554-device-20260908-20260908-082642-962`.
Its exit 1 came from the one-off runner expecting `control_ok` on
`show_conversation_home`; that existing action returns UI state instead.
Independent readback confirmed `active_surface=conversation_home` and
`activity_bound=true`. Both send samples passed; the awake setting was restored.
The E `private_completed_ms=125` field is invalid because the runner did not
require a newer stream revision for that timestamp. It is excluded from timing
claims; no extra send was run to replace it. Final stream revision was 11.

## Remaining work

The current resolver requires the mounted conversation server ID to equal the
URL conversation ID. That check is the proven rejection location for E; guest
root-route behavior is a candidate explanation, not yet a verified protocol
contract. Do not remove ownership checks or replay the message to bypass it.
First resolve guest/current-conversation ownership and explicit-action readiness
against the official runtime. Then verify one accepted private transaction.

The generation-state capability remains implemented/shipped, with its workflow
accepted but its specific runtime-reader branch unconfirmed. Remaining file
exports, new-chat confirmation and other unaccepted capabilities retain their
previous status. Google stays last; the overall Goal remains active.
