# Library Append Acceptance 1635

## Result

`capability_id: android_chatgpt_private_library_attachment_append_v1`

`code_status: implemented`, `verification_status: device_verified`,
`status: completed` for the following **ordinary consecutive-reference scope**:
production native Library selection of a TXT followed by a PDF, two ready cards,
one official-runtime text send, both documents read in the reply, and exact
post-send attachment cleanup. Do not repeat this accepted scope without a
current regression. Local-upload-after-Library and mounted/cloud variants are
not included in this completion marker.

Normal Release **1.1.1635 (1635)**, adapter **318**:

- Source: `2e5c5c3a8666644f62ab045f11fd934002abb6cc`.
- APK SHA-256: `962880129da29400350f218b95a8baea5eea5621d609f742c435ec0a8143e21c`.
- `library-policy-release-20260910-154525-121`: Gradle succeeded in 8m,
  145 tasks (83 executed, 62 cached); publication, remote hash/size and wireless
  replacement installation passed. MCP independently reported installed 1635.
- Policy correction: `fc82165a4`; bounded diagnostic: `75e2208f9`.
  See [the optional quota contract](../chatgpt-private-library-append.md#optional-quota-contract).
- The previous explicit-undefined fixture failed on the old policy; related
  checks passed **174/174**, no skips/cancellations, on the new implementation.

## Production Path

`library-policy-native-acceptance-20260910-155506-121` completed in 98.2s.
The external semantic runner used the installed consumer UI, not a retired
test page, coordinate clicks or an official-page attachment button:

1. Open the native social AI conversation and create a new test conversation.
2. Open the sidebar's Library entry, search the fixed TXT, select its native row
   and press **Attach to current chat**. Confirm receipt, ready card, remove
   control and closed Library browser.
3. Repeat those native controls for the fixed PDF. Confirm two ready cards.
4. Send exactly one controlled prompt through the production input dispatch.
   The official runtime returned `official_runtime_v1:accepted`.
5. The native reply contained the exact first-line markers from both documents.
   There was one user row, streaming ended and all selected attachments were
   consumed. The send/reply check took 41,991ms; no first-token or thermal speed
   improvement is claimed from this duration.

Both attachment receipts were `library_attachment_associated`; observed ready
counts were respectively 1 and 2. The run returned `passed: true`, `restored:
true`, and `awake_restored: true`. This supersedes 1634's failed second-file
acceptance for this scope, not its historical evidence.

## Restoration And Boundaries

The run returned to its initial empty native conversation with a current, ready
adapter and no draft or attachments. The older original conversation, retained
in process memory from the 1634 round, was then explicitly reopened. A separate
MCP read confirmed the exact original path, two displayed historical messages,
ready composer, no draft, no attachments and no streaming. Its private path and
content were not written to this report. No Library records were deleted.

No Cookie, application data, proxy, microphone/voice implementation or login was
cleared or modified. The earlier stale bridge recovered after the normal APK
replacement; its underlying pause/recovery cause has not been isolated and this
is not claimed as an automatic-reconnection fix.

The generic 1634 rejection had no per-boundary diagnostic, so it cannot establish
which exact owner check failed then. The new optional-parameter correction is
source-backed and the full new flow now passes; the independent diagnostic can
identify future runtime/owner/store/scope/limit failures without exporting props
or starting a new validation/request.

Remaining: visible catalog handles expiring after 60s, reverse-order local
uploads, real mounted materialization/download, other attachment scopes and the
rest of the broader private-interface Goal. No global completion is claimed.
