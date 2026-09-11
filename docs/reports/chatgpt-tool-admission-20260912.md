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
Normal device diagnosis is pending. This diagnostic is not a completed
Study/Canvas feature and does not change either capability's completion marker.
