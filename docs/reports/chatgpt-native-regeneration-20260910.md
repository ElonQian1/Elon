# Native regeneration acceptance, September 10

Evidence only. Capability: `android_chatgpt_official_runtime_regeneration_v1`.
Status: correction delivered, end-to-end acceptance not passed. This report does
not replace the [runtime contract](../chatgpt-official-runtime-regeneration.md).

## Production harness

Commit `ec7b94671` extends the existing external UI acceptance harness to click
the rendered production assistant retry action by its semantic selector. The
initial send reads the production `web_chat_last_send_command` receipt instead
of expecting an inline MCP receipt. `-UseCurrentNativeSurface` admits only an
already foreground, bound native ChatGPT chat and skips redundant page entry.

The test creates one isolated synthetic conversation and sends once. Success
requires the official-runtime retry receipt, a different assistant identity,
unchanged original user IDs and a completed matching reply. An unknown write
never triggers an automatic second retry. The harness restores the original
conversation and its temporary awake lease in `finally`.

## Normal 1628 result

The private-runtime initial send and completed reply passed; the actual native
retry button dispatched `regenerate_response`. Its receipt was
`official_runtime_v1:regenerate_unknown:timeout`. This is an indeterminate
post-invocation result, not evidence of an unsent request or server failure.
The original two-message conversation with an empty draft was restored.

Earlier entry failures and the older inline-receipt expectation are harness
boundaries, not additional successful retries. No repeated write was issued in
response to the unknown receipt.

## Reproduced correction

Source commit `2a01aaf6f45c2d3ec3048797ca13678e3f1758d4` separates readonly
reply ownership from the model picker's enabled state. During generation the
official model picker can be disabled; it must still identify the same owned
conversation for reply observation. Write admission remains strict.

- Red: 43 regeneration tests, 41 passed and 2 failed for owned streaming and
  completed replies while the model picker was disabled.
- Green: 109 regeneration, production-wiring and model-contract tests passed;
  zero failures, cancellations or skips. The production asset bundle parses.
- Changed account/conversation/parent and unknown picker-state cases remain
  rejected. No new network polling, write replay or credential export was added.
- Model contract is v7; regeneration contract/runtime are v5. The adapter's wire
  version remains 312 because the command schema is unchanged.

This establishes a code defect, but does not establish it as the sole cause of
the 1628 live observation timeout.

## Normal 1629 delivery and boundary

Normal release `1.1.1629 (1629)` was built, published and installed without
clearing app data or login state. Installed package version was checked.
APK SHA-256:
`be2bd06e76b466f195e388f07de7d9a2661444884cf935eb608e910ca58a8551`.

The grouped release command completed successfully in 389.6 seconds; Gradle
reported `BUILD SUCCESSFUL`. Log prefix:
`regenerate-observer-release-20260910-102958-163`.

Live acceptance log prefix:
`native-private-regenerate-1629-20260910-103731-308`.
The isolated conversation and private-runtime initial-send receipt passed, but
the completed initial-reply predicate timed out after 180 seconds. The last
snapshot was `page=conversation`, `bridge=ready`. The retry stage was never
reached, so this run neither verifies nor disproves the v5 retry correction.
The existing timeout output does not preserve enough reply-state detail to
separate missing content, incomplete state and marker mismatch. Do not infer a
network outage or a successful completed answer from this result.

The restoration/awake cleanup ran without a restoration warning. A later root
MCP read reported `conversation_home`; no additional post-run chat-content
restoration claim is made. Wireless ADB independently answered a shell probe
and device-property query after the timeout; transport disconnection was not
observed. No additional probe message or retry was sent after the failure.

## Next bounded check

Inspect the existing synthetic turn's completion state before any new send;
retain only counts, known states and matching booleans in diagnostic output.
Resolve the initial-reply boundary, then finish one real native retry acceptance.
Do not repeat already accepted tools/models, broaden to thermal tests or mark
regeneration completed based on build/install or offline tests alone. Project,
temporary, partial-turn and account-restriction scopes remain unverified.
