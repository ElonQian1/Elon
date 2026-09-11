# September 12 Runtime Compatibility

Capability: `android_chatgpt_private_runtime_bindings_v1`, resolver 16,
adapter 361. Compatibility repair, not a new HTTP sender. Device acceptance
and release are pending at this source checkpoint.

## Evidence

Normal APK 1679 observed anchor `c2675c8c-o59yc0xo7p9m3q3o.js` while its
resolver admitted only earlier builds. Native send reported
`private_fallback:template_unavailable` and `runtime_fallback:runtime_not_observed`.
The fixed synthetic reply did not complete within the acceptance deadline.
This proves a runtime compatibility gap, not that the earlier passive stream
repair caused the missing answer.

The anchor's parsed imports identify these exact public modules:

| Role | File | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-o59yc0xo7p9m3q3o.js` | `626dec60a232340efeac2f154db07232740de506b221abf5fafe0e71d4ab03a2` |
| Shared | `4813494d-gf2h57w5fiay19bd.js` | `6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e` |
| Conversation | `conversation-small-h1dtzoris1y9588z.js` | `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e` |
| Composer | `8b34dbc2-fqgb3eqijpn96umi.js` | `1d0b132fe9b13120370395bbfbe4bf3324c4dc1213b80e591cd3c6a6db7d16a1` |

Existing AST comparison found 56 unique matches out of 58 consumed declarations.
Batch helper `o2` is the only exported candidate. Actual stop function `KM`
(export `xWt`) reads async status `Uo` imported as `aC`, tree selector `GJ`
and status enum `rtt`; these dependency edges resolve the ambiguous store.
Tools owner `myn` and temporary owner `JYt` match independently. Query-client
getter `h2` matches the prior `Z0` declaration. All earlier supported profiles,
single-flight imports, document ownership and uncertain-write no-replay remain.

## Evidence Correction

The earlier Canvas save bridge mistook imported `HH as one` for an export.
The real edit queue is local and unexported in both reviewed builds. The bad
mapping is removed, not guessed under another alias. Save/history/first-share
still reject before a write when that queue cannot be inspected. Shared Canvas
read/view/copy results remain separate and are not invalidated by this finding.
See [corrected save evidence](chatgpt-original-canvas-save-20260912.md).

The new `test-chatgpt-runtime-public-evidence.cjs` checks the pinned module
hashes, anchor dependencies, actual export table and stop/temporary owners.
It parses but never executes downloaded website code. Set
`CHATGPT_PUBLIC_RUNTIME_DIR` and `CHATGPT_AST_PARSER` to the local reviewed
assets and Acorn installation; ordinary offline runs skip this external-source
check and continue using the committed behavioral fixtures. Mock consumer tests
alone are not public-protocol proof or device acceptance.

## Verification

- Pinned public-source check and versioned binding tests: 81 passed, none skipped.
- Submit/stop/attachment/regeneration/stream checks: 333 passed.
- Current-profile consumers and committed-owner checks: 25 passed.
- Grouped release and one production native send/reply remain pending.
No Cookie, application-data, microphone or proxy-core changes were made.
