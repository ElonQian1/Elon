# ChatGPT Tool Admission Diagnosis

## Scope

Follow-up to the actual 1671 native Study selection failure recorded in
`chatgpt-study-canvas-tools-20260911.md`. The installed catalogue had Search and
Image Generation, although the APK contained Study and Canvas definitions.
This is not evidence that the provider lacks either capability.

## Implementation

Adapter 356 extends the existing passive composer-context observation. The
existing MCP `chatgpt_private_protocol_probe` mode `composer_tool_admission`
returns `elon.composer_tool_admission.v1`: whether capture was observed and, for
at most four known tools, bounded raw/menu match counts and a fixed exclusion
reason. Reasons distinguish missing hints, menu filtering, ambiguity, upsell,
different tool kind, initial hiding, disabled state and different transaction.

The diagnostic never captures a new context, changes a tool, loads a module or
starts a network observer. It exports no model/account identity, conversation
text, path, credentials or headers. Document-token changes invalidate the
observation; consumers receive a copy. Native validation rejects extra fields,
unknown tools/reasons, duplicate rows and non-integer/out-of-range counts.

Existing admission and selection policy is unchanged. Current inspected
official code treats Study (`tatertot`) and Canvas (`canvas`) as ordinary hints;
the separate hidden/local-action resolver is for Sketch. Actual live eligibility
must still be checked before changing the integration. No eligibility bypass,
new endpoint, alternate state owner or uncertain-write replay is introduced.

## Verification

Two focused Node suites: **103 passed**. Android Release production/test
compilation succeeded; **19 targeted Kotlin tests passed**, verified from the
JUnit XML results (admission, protocol evidence/MCP, Canvas shared links).
This diagnostic is not a completed Study/Canvas feature and does not change
either capability's completion marker.

## Normal 1672 Device Evidence

Source `f5229dce2`, adapter 356, normal **1.1.1672 (1672)** was published and
installed on the pinned Xiaomi with `adb install -r`. Package version readback
matched. APK SHA-256:
`08bf2203520c1e36c4fbecdd5d5c89bc24c26f4d81df6e63932a493b583a27b0`.
Publisher verified remote hash/size; the existing postflight cleanup reported a
missing `Branch` property warning, separate from successful release/install.

Existing native MCP returned `official_tool_runtime_v1:accepted` and
`composer_tool_context:ready`. Search and Image Generation each had one raw and
one menu hint. Study and Canvas both had zero raw/menu hints (`raw_missing`).
This held for the original Extra High choice and a reversible Instant comparison.
No message was sent; the conversation, empty draft and awake policy were retained.
It narrows the current catalogue gap to the official producer context, not our
disabled/hidden filtering. It does not prove global provider unavailability.
Log groups: `tool-admission-device-1672`, `tool-model-compare-1672`.

The Instant comparison exposed another regression: subsequent model capture
reported `trigger_detached`, and no model options were returned. Immediate
restoration was not confirmed. After one idle same-conversation refresh, a fresh
private model list confirmed the original Extra High choice selected. Do not
describe the initial comparison's `model_restored=false` as a pass.

## Instant Label Repair

The real private catalogue labels Instant as `\u5373\u65f6`. The shared model
label policy did not recognize it. A new test uses the actual production composer
finder, the real label policy, multiple actionable composer buttons and an
unrelated header model control. Before the repair it selected the wrong header;
the label unit case also failed. Adapter 357 adds only this observed label and
its repeated accessible-label form. It does not loosen runtime identity/model
guards, scan arbitrary message text or add another model store.

All **90 focused Node cases passed** after the repair, including private model
selection and restrictions. This is a scoped repair within the existing model
capability. Updated device selection/restore verification is pending;
Study/Canvas remain incomplete.
