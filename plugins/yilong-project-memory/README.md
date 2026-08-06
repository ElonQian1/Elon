# Yilong Project Memory Codex Plugin

This plugin exposes three separate, least-privilege MCP servers backed by the local Yilong node:

- `yilong-project-context` provides one bounded, read-only `project_context_plan` tool.
- `yilong-project-features` provides one on-demand `project_feature_workflow` dispatcher; its detailed action schemas are returned only by an explicit `describe` action.
- `yilong-project-memory-receipt` provides one review-gated candidate receipt tool.

The plugin never reads or copies Codex private memories. It stores no source bodies, prompts,
chat transcripts, command text, or tool output. The optional hooks retain only normalized relative
paths and access kinds under the dedicated `PLUGIN_DATA/session-ledgers` directory, remove the
session ledger on `SessionEnd`, and expire stale ledgers after 24 hours.

Requirements:

- Node.js 18 or newer is available as `node`.
- The Yilong Windows node is running on its loopback admin port, or `ELON_NODE_ADMIN_URL` points to it.
- The plugin starts each MCP process with the installed plugin root as its explicit working
  directory and forwards only `ELON_NODE_ADMIN_URL` and `ELON_PROJECT_ROOT` from the host.
- `ELON_PROJECT_ROOT` names the target Git project. The proxy can still walk upward from its cwd
  for development checkouts, but a cached plugin install is not inside the target project, so
  desktop/CLI integrations should provide the explicit project root.

Installing or enabling the plugin does not trust its hooks. Review the hook definitions in Codex
before enabling them. Current source, tests, binding rules, and current ADRs always outrank shared
navigation memory.

Codex executes a cached copy, not the marketplace source directory. After changing any file in
this plugin, increment the plugin version, remove the installed copy, add it again from the local
marketplace, and start a new task. Repeating `add` after changing only SemVer build metadata can
leave an older equal-precedence cache selected; the repository readiness script detects a cache
that no longer matches the source.

The context plan can also project at most three related, Git-backed feature records without
returning requirement bodies. The separate feature dispatcher can register, update, rebind, claim,
or transition an explicit feature without exposing the full document-governance catalog to every
task. It uses the same server-side state machine and never returns requirement or source bodies.
