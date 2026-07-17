# Platform capability gap protocol

Use a platform gap only when the current Yilong tools cannot represent, render, measure, commit, or verify the requested UI outcome. A normal Android structure change is not a platform gap.

Required evidence:

- Original UI task ID and optional FitRun ID.
- Missing capability from the controlled capability names returned by `ui_check_capabilities`.
- The tool call or renderer evidence that failed.
- The smallest proposed platform change.
- A resume target describing the original UI step.
- `executionMode`, `deliveryImpact`, and the originating Codex thread ID.
- For `DELIVERY_NON_BLOCKING`, the final Git source revision, exact FitRun ID, visual/source-parity losses and thresholds, deterministic source write-back, patch-free build proof, and a connected Runtime source proof with no live Patch/redo history.

Trusted boundary:

- The skill and project MCP configuration are distributed with this repository.
- Automatic source upgrade and publish apply only to the current canonical local Git workspace.
- A different repository has neither this skill nor a project-scoped MCP session and cannot mutate Yilong source.

Delivery impact:

- `DELIVERY_BLOCKING`: the user's requested UI cannot yet be represented, rendered, measured, written, or verified. The business task pauses after creating a separate evolution task.
- `DELIVERY_NON_BLOCKING`: the user's requested UI is already source-written and patch-free verified within the visual thresholds. Only platform efficiency, automation, resilience, or reuse remains. `businessDeliveryReady=true` lets the business task finish without claiming full platform closure.
- `EVOLUTION_ONLY`: valid only inside the delegated platform task. It cannot recursively create another business handoff.

Lifecycles:

`BUSINESS_THREAD: DEFERRED -> CODEX_DESKTOP_WORKTREE_HANDOFF`

`EVOLUTION_THREAD: APPROVED -> UPGRADING -> PUBLISHED -> COMPLETED -> NOTIFY_ORIGIN`

The business thread never enters `UPGRADING`. Its report returns a portable `threadHandoff` containing the prompt and `reportArguments`; after business finish, Codex Desktop creates a user-visible project Worktree task and returns control to the user without waiting. The trusted evolution Worktree authorizes `APPROVED` without a separate browser approval. It must report the before/after source revision, unique commit, release version, and changed files. Empty releases, duplicate commits, repeated failure signatures, and exhaustion of eight upgrade rounds move the evolution gap to `HUMAN_REQUIRED`. A successful recheck completes the evolution gap and notifies the origin task.

Resource priority:

- Foreground UI tasks always outrank background evolution.
- Real Android renderer/device leases, node-agent publish, and node-agent restart are serialized resources. Background evolution waits without holding them while foreground UI work is active.
- Device authorization failures require human action; do not create repeated debug application IDs or reinstall loops.

Capability derivation:

- `ui_check_capabilities` unions caller-declared requirements with requirements derived from the structured task and project profile. A caller list is additive and cannot suppress a known platform gap.
- A project profile with `apkWebUiParityRequired=true` always derives `CROSS_PLATFORM_STYLE_WRITEBACK` for UI work.
- `ui_check_workflow_completion` exposes two independent claims: `completionReady` for full platform closure and `businessDeliveryReady` for verified business delivery with a non-blocking evolution pending.

Cross-platform verification contract:

- A supported `CROSS_PLATFORM_STYLE_WRITEBACK` implementation writes `cross-platform-verification.json` inside the current task directory.
- The schema contains `schemaVersion=1`, the same `taskId`, the current `sourceRevision`, existing `androidArtifact` and `webArtifact` files, finite `visualLoss <= maxVisualLoss`, `sourceWritebackVerified=true`, and `patchFreeBuildVerified=true`.
- Source or syntax checks alone are not visual parity evidence.
