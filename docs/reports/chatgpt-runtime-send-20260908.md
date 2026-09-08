# ChatGPT runtime text delivery evidence

Date: 2026-09-08. Capability:
`android_chatgpt_official_runtime_text_submit_v1`.

## Accepted scope

- `guest_plain_text_fresh_and_continuing`: **completed**, device verified on
  the production native social AI surface of APK 1559. Reuse this implementation;
  do not repeat this research without a current regression.
- This is the official page-runtime submit transaction with native UI, not an
  independent Android HTTP POST sender. The website still owns dispatch proof
  and its conversation/editor state. No ambiguous write is automatically replayed.
- Authenticated/project/tool/attachment variants, independent POST and precise
  latency/thermal improvements are not proven by these two guest text samples.

## Implementation

The bridge uses the mounted official composer and exact versioned exports.
Current-draft submission is separate from explicit-action readiness and reads
the current editor document, not a delayed UI subscription. Plain text dispatch
is acknowledged only when official completion resolves exactly true; a later
editor reset cannot revoke that acknowledgement or authorize clearing another
draft. Attachments retain their previous stricter cleanup contract.

Guest mode is established by the official logged-out bootstrap and null live
session, separately from request credentials. A credential-bearing guest can
retain the exact current conversation on the homepage. Credentials, document,
controller, thread and draft remain revalidated before dispatch. The added
regression failed against the previous code. Other consumers do not opt into
the guest homepage exception; no login or policy constraint is bypassed.

## Delivery

| APK | Source | Runtime / bindings / orchestrator | Build / publish |
| --- | --- | --- | --- |
| 1558 | `02b487f4dee7dde374706c29d72062166d8390b9` | 14 / 4 / 7 | 6m57s / 473.3s |
| 1559 | `ffc821d17f62b0ca839718d7dbed28ff492084ab` | 15 / 4 / 7 | 5m10s / 350.3s |

Both use adapter 299. Release build, publication, remote hash verification and
pinned Xiaomi replacement installation passed. Packaged JS matched source after
newline normalization. No Cookie, app data, login, VPN or proxy core was changed.

APK SHA-256:
- 1558: `1b00a110de7015cc83371830f27ae69dfbb66cf0fd9a39832a452c3d04308f05`
- 1559: `de96c22a5464bcbd970b4a764cc9f3fd9ef0b66ff14591144ea3e81cf8665b99`

279 integrated Node cases passed for the acknowledgement repair; 282 passed
after guest-credential separation. Source/ownership/document gates passed.
No new voice or attachment device acceptance is claimed.

## Production results

All samples used native APK MCP input/send handlers, fixed synthetic prompts,
idle voice and empty draft guards. No microphone, screenshot or official-page
input action was used. Each turn was sent exactly once.

| APK / turn | Runtime receipt | Complete reply observed | Native settled |
| --- | --- | --- | --- |
| 1558 / first | accepted | 5,533 ms | 5,544 ms |
| 1558 / continuing | DOM fallback: conversation_route_mismatch | 9,108 ms | 11,280 ms |
| 1559 / first | accepted | 4,502 ms | 4,512 ms |
| 1559 / continuing | accepted | 2,484 ms | 4,561 ms |

Each row has one exact user fixture and one exact assistant reply. Both 1559
rows have fresh successful runtime receipts, no fallback code, new completed
stream revisions (5 and 9), and native streaming false. Two-second polling
limits timing resolution; this is not a controlled performance A/B.

The first 1559 preparation run timed out waiting 35 seconds for the blank native
conversation and sent no text. Read-only follow-up found it empty and ready.
Acceptance then used that existing blank conversation without repeating creation.
Cold/new-conversation readiness remains an explicit unresolved gap.

The phone returned to conversation_home with its test-time awake setting 15
restored. Earlier 1555 cleanup hit a PowerShell HOME-name collision and failed
to restore that setting; recovery used the recorded system default 15, not an
unrecorded original value. The runner now uses a nonreserved name and nested
finally cleanup. Earlier 1555/1556 unknown receipts are not counted as accepted.

## Logs and follow-up

Logs: `runtime-text-receipt-final-20260908-20260908-094128-540`,
`runtime-guest-credential-final-20260908-20260908-100744-064`,
`runtime-text-receipt-release-20260908-20260908-095049-274`,
`runtime-guest-continuation-release-20260908-20260908-101125-117`,
`runtime-text-1558-device-20260908-20260908-095935-811`,
`runtime-guest-1559-device-20260908-20260908-101757-334`, and
`runtime-guest-1559-ready-device-20260908-20260908-102111-754`.

Optional LAN firewall setup and broad auto-cleanup emitted warnings; mandatory
task finalization is separate. The overall Goal stays active. Resolve the
recorded new-conversation readiness delay and remaining ChatGPT scope gaps
before Google; do not reopen the completed guest text path speculatively.
