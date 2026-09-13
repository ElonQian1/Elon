# Command Recovery And Same-Route Readiness

Date: 2026-09-13. Evidence report, not a new product requirement.

## Grouped Release

- APK 1.1.1698 / code 1698, source `746c2be79b5cfbefd751e830ba6484dd9337ce12`.
- APK SHA-256: `6dee6ee498beabf9199ff43f7826520df85f11d1c54a07015496c5309caa6673`.
- Publisher `chatgpt-delivery-grouped-release-20260913-093235-295` passed
  in 413.9 seconds. Online metadata and installed Xiaomi package matched.
- Includes shared native/MCP request sequence and proven-unsent command repair.
  Cookies, login, application data and VPN settings were not cleared.

## Device Evidence

The native `social_ai` surface passed cold and background-return read-only
`document_state` commands in 152 and 685 ms respectively. These include MCP
automation overhead, not network or first-token latency. Both reported document
complete and a ready composer. Page generation remained 1; the conversation,
draft and keep-awake setting were restored. No messages were sent.

Delivery traces showed invalidations, not `repair_started`/`repair_delivered`.
The live result therefore does not exercise the forced missing-bridge branch
or prove that branch caused every older timeout.

Stop/follow-up logs `fresh-text-stop-followup-native-1698-20260913-094257-908`
and `fresh-text-stop-followup-ready-1698-20260913-094658-023` both failed at
`opening`, with zero seed sends and zero candidate clicks. Restoration and
keep-awake cleanup completed. The second run followed a read-only check proving
the owned fixture route and both native/page readiness matched. No stop or
follow-up acceptance is claimed.

## Reproduced Source Defect

Opening the current conversation first projects native loading state with
`composerReady=false`. The page intentionally avoids navigating the same exact
route. Its full snapshot was identical to the previous one, so fingerprint
deduplication suppressed the state needed to restore native readiness. A
content-only private prefetch does not restore composer state. The open receipt
can succeed by route identity while the native view remains loading.

The adapter now sends one explicit full snapshot for this exact same-route
open. It still samples the real composer and never fabricates readiness. Normal
scheduled snapshots remain deduplicated. Different routes, query/fragment
navigation, stale document commands and rejected drafts do not use this override.
No new timer, reload, HTTP retry or message replay was introduced. Adapter 376
replaces the prior adapter through the existing versioned bootstrap.

## Verification

`test-chatgpt-web-same-conversation-snapshot.cjs` runs the actual production
adapter with bounded page substitutes. Before the fix, three readiness cases
failed; afterward all nine passed. The combined navigation, scheduler, draft,
command-delivery, Writing Block save/parser, private-stream and fresh-text wiring
regression passed 122 tests with no skips in
`chatgpt-same-route-regression-20260913-095933-238`.

The same-route correction is source verified and awaiting the next package and
one targeted device reopen. The broader Goal and stopped-turn follow-up
acceptance remain open. Existing Writing Block/code-block acceptance is unchanged.
