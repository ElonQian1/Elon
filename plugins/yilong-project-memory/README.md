# Yilong Project Memory Codex Plugin

This plugin exposes two separate, least-privilege MCP servers backed by the local Yilong node:

- `yilong-project-context` provides one bounded, read-only `project_context_plan` tool.
- `yilong-project-memory-receipt` provides one review-gated candidate receipt tool.

The plugin never reads or copies Codex private memories. It stores no source bodies, prompts,
chat transcripts, command text, or tool output. The optional hooks retain only normalized relative
paths and access kinds under the dedicated `PLUGIN_DATA/session-ledgers` directory, remove the
session ledger on `SessionEnd`, and expire stale ledgers after 24 hours.

Requirements:

- Node.js 18 or newer is available as `node`.
- The Yilong Windows node is running on its loopback admin port, or `ELON_NODE_ADMIN_URL` points to it.
- Codex starts the MCP process inside the target Git project, or `ELON_PROJECT_ROOT` names that
  project explicitly. The proxy walks upward from its cwd to find the nearest `.git` marker.

Installing or enabling the plugin does not trust its hooks. Review the hook definitions in Codex
before enabling them. Current source, tests, binding rules, and current ADRs always outrank shared
navigation memory.
