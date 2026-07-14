# Platform capability gap protocol

Use a platform gap only when the current Yilong tools cannot represent, render, measure, commit, or verify the requested UI outcome. A normal Android structure change is not a platform gap.

Required evidence:

- Original UI task ID and optional FitRun ID.
- Missing capability from the controlled capability names returned by `ui_check_capabilities`.
- The tool call or renderer evidence that failed.
- The smallest proposed platform change.
- A resume target describing the original UI step.

Trusted boundary:

- The skill and project MCP configuration are distributed with this repository.
- Automatic source upgrade and publish apply only to the current canonical local Git workspace.
- A different repository has neither this skill nor a project-scoped MCP session and cannot mutate Yilong source.

Lifecycle:

`APPROVED -> UPGRADING -> PUBLISHED -> RESUMED`

The trusted local Git workspace authorizes the transition to `APPROVED` without a separate browser approval. Codex must report the before/after source revision, unique commit, release version, and changed files. Empty releases, duplicate commits, repeated failure signatures, and exhaustion of eight upgrade rounds move the gap to `HUMAN_REQUIRED`. A successful recheck must resume the linked UI task instead of ending after platform publication.
