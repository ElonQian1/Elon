# ChatGPT stop and follow-up, APK 1576

## Delivered

APK `1.1.1576` / code `1576`, source
`80b65b40f09ffeee4e6000aab0946e92dd593f24`, was built and published through the
standard publisher, then installed with `adb install -r`. The live version
receipt matches the local APK SHA-256:
`0159dcff80912b006c5a05322ec73de1e20fb6a1e5c71af4f35d7c8340c4af24`.
No application data, Cookie, identity or independent proxy setting was cleared.

Build log: `stop-composite-request-release-20260908-192440-426`.
Install log: `stop-composite-request-install-20260908-193408-708`.
The 129 focused stop, request ownership and diagnostic cases passed before
publication; see [the capability record](../chatgpt-official-runtime-stop.md).

## Device result

The production `social_ai` native ChatGPT UI on the authorized Xiaomi, adapter
306, remains a guest session. Synthetic native send, mid-response stop and
follow-up were exercised without microphone access or private content output.

- Both sends were acknowledged through `official_runtime_v1:accepted`.
- Stop returned `official_runtime_v1:stop_observed`, not DOM fallback.
- Runtime owner reported cached tree, a valid request and streaming mode at
  capture. The previously rejected composite request format is resolved.
- The first run retained all 344 observed characters immediately after stop.
- Follow-up still produced one user row and one assistant row, not four rows.
- Native and official drafts were empty before and after native submissions.
- A shape-only follow-up check confirmed that the single user row contains
  both test prompts: lengths 85 and 28 joined by two newlines, total 115.
  The final assistant row has 11 characters and no retained partial prefix.
- Both native-run harnesses restored the original empty view and awake lease.

Evidence: `stopped-turn-device-1576-20260908-193445-211` and
`stopped-turn-shape-1576-20260908-194040-169`.
These results prove the native/web projections are combined, not whether the
official underlying conversation tree discarded a turn. Do not label that
unobserved tree conclusion as established.

A later official-send-control comparison was inconclusive, not a passing
official baseline: the temporary harness initially used `id` instead of the
returned `control_id`, then a correctly tracked click did not dispatch a send.
The retained 28-character synthetic official draft was subsequently cleared;
no uncertain send was replayed. Its original harness reported restoration
false. Evidence: `stopped-turn-official-control-1576-20260908-194556-439`.
After that controlled draft cleanup, MCP confirmed the original empty native
view was restored: zero messages, zero native/official draft lengths and no
streaming. The comparison remains inconclusive despite successful cleanup.

## Stream visibility correction

The existing private stream policy accepted any assistant-role text as an
answer. It did not check message channel, recipient or explicit visibility
metadata. This is an independent source-level gap and a possible confounder
for an acceptance gate that stops at the first 80 assistant characters.
It has not been established as the phone's follow-up root cause.

Public source evidence, using the pinned 2026-09-07 cache:
`conversation-small-owrec55n6vm0ekcc.js`, SHA-256
`7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
Its `Vgr` excludes analysis and explicit hidden/debug reasoning from visible
summaries; the adjacent assistant-answer predicate restricts recipient to
absent/all and channel to absent/final. Protocol role alone is insufficient.

The correction filters non-final channels, tool recipients and the three
observed hidden/debug flags before answer text, stream identity or widgets
are accepted. Existing channel-less legacy answers and final text still stream;
allowlisted public search progress retains its separate path. DOM rendering is
not changed. There is no request, prompt, account or voice-path mutation.

New synthetic visibility tests failed against the preceding source. The
visibility, stream-policy and transport test run passes 14 Node test entries,
including the existing policy/transport assertions. Evidence:
`stream-visibility-red-20260908-195409-439` and
`stream-visibility-green-20260908-195433-006`.
The filtering correction is source-tested but not yet included in APK 1576.
The expanded focused regression passes 34 Node entries with no skips, covering
interruption retention, streaming/watchdog behavior and Win binding/recovery:
`stream-visibility-regression-20260908-195557-267`. Source commit
`697be9e45650fcc3d24098c0814c2ccfa8cc7cdf` is pushed to `origin/main`.
Per the grouped-build workflow, it is queued for the next APK batch rather
than another one-change build. No device acceptance is claimed for that filter.

Next acceptance must start from the actual final-answer stream and separately
establish official-tree and native projection continuity. Do not replay already
confirmed stop requests or mark the full capability completed from stop alone.

## Grouped filter delivery, APK 1578

APK `1.1.1578` / code `1578`, source
`4dcb4b658004f86150ea5ce8dd28c88c2d43d3b6`, includes the public-answer filter.
The standard Release build and publication passed in 455.2 seconds. Local APK
and live version endpoint SHA-256 agree:
`1089caf313b667241e6921a3fc37fad094c77590096f1bec48ed8ed53e86a227`.
Evidence: `public-answer-filter-release-20260908-202020-354`.

The trusted Xiaomi ADB transport was offline, one bounded reconnect timed out,
and the post-release device list was empty. No replacement install or device
acceptance was performed. The last confirmed installed version from this task
remains 1576; the latest actual phone version is unknown while disconnected.
The 1576 continuity failure remains open. Publishing the independent filter
does not prove that it resolves that failure.

The durable `smoke-chatgpt-web-stopped-followup.ps1` uses native input/send and
typed stop receipts, requires a recognizable public-answer prefix before stop,
and checks ordered distinct user/assistant turns with the original partial
answer retained. It refuses existing drafts, streaming work and unrestorable
guest history. Foreground/provider/work-mode changes abort the test. Cleanup
cannot overwrite the primary failure or navigate away from unresolved work.

Its first source-only native guard incorrectly checked `surface` rather than
the main snapshot's `active_surface`. A synthetic production-shape check failed;
the corrected guard reuses `Get-ChatGptWebNativeChatState`. All 11 evidence and
surface checks now pass, including merged/lost/unfinished/empty turns and
foreign foreground/provider/mode. This harness correction is not APK runtime
code and does not require another package. No repeat of the unchanged 34-case
stream regression or already accepted private capabilities was performed.
