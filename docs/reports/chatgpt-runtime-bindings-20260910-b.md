# September 10 second runtime admission

Capability: `android_chatgpt_private_runtime_bindings_v1`, resolver 11,
adapter 325. This is compatibility repair for existing consumers, not a new
HTTP sender or completion of every private feature.

## Device evidence

Normal APK 1639, adapter 323, acknowledged a fresh production-page diagnostic
at document generation 6. The original conversation was restored and confirmed
by the webpage, not merely a cached native selection. A model catalog request
succeeded, but its following diagnostic reported
`model_runtime_context:runtime_not_observed`: a successful legacy catalog is
not private-runtime admission. The page's public asset inventory identified
the new build below. No message or regeneration was dispatched in this read.

This explains the current pre-invocation admission failure. It does not prove
the cause of the earlier 1638 post-dispatch `timeout_owner_changed` failure.
Its ownership fences remain unchanged.

## Public source contracts

The exact phone-observed assets were downloaded without credentials from
`https://chatgpt.com/cdn/assets/`. The anchor's parsed static imports identify
all three roles and unchanged React `2340486e-dyt4epctwx2pn2sj.js`.

| Role | Filename | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-k523hcgvefxht2ry.js` | `c64a770a194ced90643b56b3dc1880c39b2cf9889bc6233e709ff616d99d50e5` |
| Shared | `4813494d-fgzk5uadh2hlkw93.js` | `853f1aec75ae5d51c679e0bbf2e8b62c0b514f81a2f1f2cbd24b0e47ecba6b80` |
| Conversation | `conversation-small-cudd01juo7e4yskq.js` | `c3c57597bf6202630080f3e8f53cdb356175dd2ce646b0f3bed3d16a8970bea6` |
| Composer | `8b34dbc2-g959q8r0i61e6rlk.js` | `f8106fe4855449301505c67b49b885cafff21f81c22542ceb28f75f24695bd23` |

Acorn parsed the previous admitted September 10 assets and this build without
executing provider code. Normalization preserves literal values, object/member
keys and operators while ignoring local identifier spelling. Of 57 consumed
contracts, 54 have unique declaration matches. The batch helper has two matching
declarations but only one exported candidate (`og`, export `I0`).

Two contracts needed additional evidence:

- Seven structurally identical async-store candidates are not interchangeable.
  The actual current stop function `kN`, export `KHt`, imports `Ze` as shared
  `BS`, active-request `ee` as `$u`, and state enum `Yr` as `Oet`. These identify
  the correct legacy `Fx`, `Fl`, `v7` bindings.
- Composer `fh` maps to `Ph` / `CB`, the attachment store referenced by the
  actual temporary-chat owner. Of its 23 methods, only `uploadFile` differs;
  the consumed `validateChatAttachment` and all other methods match. This
  adapter consumes its validator, not the changed upload implementation.

Tool owner `$vn` and temporary owner `fYt` each uniquely match their preceding
owner; their 265/30-slot contracts are unchanged. The exact temporary action
fingerprint is independently recorded in the new fixture. Runtime selection
retains all five earlier profiles, exact observed-file matching, document-bound
singletons, import coalescing/cooldown and uncertain-write no-replay rules.

## Verification boundary

Before the fix, the new-profile regression run failed 11 of 35 cases. After the
fix, all 132 binding/current-consumer/regeneration/committed-owner cases passed;
another 304 model/tool/temporary/attachment cases passed. No skips or cancellations.
Logs: `runtime-sep10b-green-20260910-204358-367` and
`runtime-sep10b-consumers-20260910-204447-106`.

Source comparison and its public assets are retained in the repository's external
research artifacts. Offline fixtures prove composition and guards, not live
provider acceptance. Existing verified voice transports were neither modified
nor repeated. No thermal improvement is claimed.

## Grouped normal release 1640

Source `c6e5df908` passed Release and lint, publication and unattended `install -r`
on the trusted Xiaomi as `1.1.1640 (1640)`, adapter 325. It also includes the queued
[citation resolver correction](../chatgpt-private-file-citations.md).
APK size is 40,116,617 bytes; local and remote SHA-256 agree:
`d1d4b56e0e4fe1dbeac079c2d1bb06907df6add99afbc19ee0713f335322f9a8`.
The packaged asset was read back and contains resolver 11 and the new profile.
Log: `runtime-sep10b-grouped-release-20260910-204951-526`, terminal pass, 601.6s.

Production model acceptance stopped at the locked-device preflight, before any
catalog request, navigation or retry: `runtime-325-model-admission-20260910-210021-880`.
The phone's installed version is verified; private catalog, regeneration and
citation-download acceptance remain deferred, not failed provider calls or
successful private transactions. Unlock is the next device prerequisite.
No data/Cookies were cleared, no microphone was opened and no message was sent.

The first launch wrapper rejected shell invocation syntax before starting the
publisher (0.4s); the corrected explicitly hidden `pwsh -File` invocation above
was the only build. Publication also reported an optional LAN firewall warning
and a worktree-cleanup `Branch`-property warning; neither invalidates the verified
APK installation. Main/worktree cleanup is handled separately by task finish.
