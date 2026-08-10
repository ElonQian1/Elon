package com.elon.app.mcp

import com.elon.app.*
import org.json.JSONArray
import org.json.JSONObject

internal fun mcpInitializeResult(protocolVersion: String, port: Int, tag: String): JSONObject {
    return JSONObject()
        .put("protocolVersion", protocolVersion)
        .put("capabilities", JSONObject().put("tools", JSONObject().put("listChanged", false)))
        .put(
            "serverInfo",
            JSONObject()
                .put("name", "elon-phone-debug")
                .put("title", "Elon Phone Debug MCP")
                .put("version", BuildConfig.VERSION_NAME)
        )
        .put(
            "instructions",
            "Use adb forward tcp:$port tcp:$port, then call tools with the token from GET /health or logcat tag $tag."
        )
}

internal fun mcpToolsListResult(): JSONObject {
    return JSONObject().put(
        "tools",
        JSONArray()
            .put(
                tool(
                    name = "phone_status",
                    title = "Phone Status",
                    description = "Return app, MCP server, active project, and pending task state from the phone.",
                    properties = JSONObject(),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "trace_recent",
                    title = "Recent Trace Events",
                    description = "Return recent persisted phone trace events written to logcat tag ElonTrace, optionally filtered by trace id, phase, text, or wall time.",
                    properties = JSONObject()
                        .put("limit", intProperty("Maximum events to return, 1-300."))
                        .put("trace_id", stringProperty("Optional trace id filter."))
                        .put("phase", stringProperty("Optional exact phase filter."))
                        .put("contains", stringProperty("Optional case-insensitive text filter across phase and details."))
                        .put("since_wall_time_ms", intProperty("Optional lower bound for event wall_time_ms.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "trace_clear",
                    title = "Clear Trace Events",
                    description = "Clear the in-memory phone trace buffer.",
                    properties = JSONObject(),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "debug_session",
                    title = "Debug Session",
                    description = "Start, end, or inspect a named MCP debug session marker so later diagnostic bundles can return only the relevant trace window.",
                    properties = JSONObject()
                        .put("action", stringProperty("One of start, status, or end. Defaults to status."))
                        .put("session_id", stringProperty("Optional caller-provided session id for action=start."))
                        .put("note", stringProperty("Optional human note stored with the session.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "diagnostic_bundle",
                    title = "Diagnostic Bundle",
                    description = "Return one compact APK debug bundle: status, self-check, device snapshot, network check, task status/events, filtered trace, logcat, and metrics.",
                    properties = JSONObject()
                        .put("start_session", booleanProperty("Start a debug session before collecting the bundle. Defaults to false."))
                        .put("session_id", stringProperty("Optional debug session id when start_session=true."))
                        .put("note", stringProperty("Optional debug session note when start_session=true."))
                        .put("since_wall_time_ms", intProperty("Only include trace events after this wall time. Defaults to the active debug session start."))
                        .put("trace_limit", intProperty("Maximum trace events to return, 1-300. Defaults to 80."))
                        .put("timeline_limit", intProperty("Maximum latency timeline events to return, 1-300. Defaults to 80."))
                        .put("trace_id", stringProperty("Optional trace id filter for trace_recent."))
                        .put("phase", stringProperty("Optional exact phase filter for trace_recent."))
                        .put("contains", stringProperty("Optional trace text filter."))
                        .put("include_logcat", booleanProperty("Include filtered logcat. Defaults to true."))
                        .put("logcat_line_count", intProperty("Raw logcat lines to scan, 20-1000. Defaults to 240."))
                        .put("logcat_pattern", stringProperty("Regex filter for logcat. Defaults to Elon/crash/foreground-service tags."))
                        .put("include_network_check", booleanProperty("Include backend network probes. Defaults to true."))
                        .put("include_update_check", booleanProperty("Include server version.json in self-check. Defaults to false."))
                        .put("include_server_trace", booleanProperty("Fetch backend trace events for the selected trace_id. Defaults to true."))
                        .put("server_trace_limit", intProperty("Maximum backend trace events to return, 1-300. Defaults to 120."))
                        .put("server_url", stringProperty("Optional backend base URL. Defaults to http://43.139.149.158:8080."))
                        .put("task_event_limit", intProperty("Maximum queued task events to return, 1-120. Defaults to 20.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "device_snapshot",
                    title = "Device Snapshot",
                    description = "Return local device/app runtime facts useful for debugging: memory, battery, network capabilities, build info, and keepalive status.",
                    properties = JSONObject(),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "network_check",
                    title = "Network Check",
                    description = "Probe backend HTTP endpoints and TCP reachability from inside the APK process.",
                    properties = JSONObject()
                        .put("urls", arrayProperty("Optional HTTP URLs to probe. Defaults to server /health and /app/version.json."))
                        .put("tcp_host", stringProperty("Optional TCP host to probe. Defaults to 43.139.149.158."))
                        .put("tcp_port", intProperty("Optional TCP port to probe. Defaults to 8080.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "background_debug_status",
                    title = "Background Debug Status",
                    description = "Return whether MCP is likely to remain reachable while the user switches to WeChat or another app, including keepalive, notification permission, battery optimization, and network validation.",
                    properties = JSONObject(),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "latency_report",
                    title = "Latency Report",
                    description = "Build a per-trace latency timeline and bottleneck summary for phone chat/development tasks.",
                    properties = JSONObject()
                        .put("trace_id", stringProperty("Optional trace id. Defaults to the active or latest traced phone task."))
                        .put("timeline_limit", intProperty("Maximum timeline events to return, 1-300. Defaults to 80.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "server_trace",
                    title = "Server Trace",
                    description = "Fetch backend debug trace events for the phone trace_id so Codex can compare phone timing with server receive/queue/reply timing.",
                    properties = JSONObject()
                        .put("trace_id", stringProperty("Optional trace id. Defaults to the active or latest traced phone task."))
                        .put("limit", intProperty("Maximum backend trace events to return, 1-300. Defaults to 120."))
                        .put("server_url", stringProperty("Optional backend base URL. Defaults to http://43.139.149.158:8080.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "mcp_self_check",
                    title = "MCP Self Check",
                    description = "Run a one-shot local health check for MCP server reachability, background keepalive, trace storage, queued events, and request metrics.",
                    properties = JSONObject()
                        .put("include_update_check", booleanProperty("Also fetch server version.json. Defaults to false.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "mcp_metrics",
                    title = "MCP Metrics",
                    description = "Return MCP request counters, active connections, last RPC/tool names, and last request error.",
                    properties = JSONObject(),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "debug_keepalive",
                    title = "Debug Keepalive",
                    description = "Start, stop, or inspect the foreground debug keepalive service so MCP stays reachable while the user is in another app.",
                    properties = JSONObject()
                        .put("action", stringProperty("One of start, stop, or status. Defaults to status.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "ui_state",
                    title = "Native UI State",
                    description = "Return native APK navigation, project, conversation, input, and runtime state without screenshots, UIAutomator, or XML scraping.",
                    properties = JSONObject(),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "ui_control",
                    title = "Native UI Control",
                    description = "Control project chat, social AI chat, and the ChatGPT Web mirror through stable native actions and semantic control ids.",
                    properties = JSONObject()
                        .put("action", stringProperty("Includes open_main, state, project actions, open_social_ai_chat, open_chatgpt_web, set_input_text, send_input, and chatgpt_* actions returned by ui_state."))
                        .put("project_id", stringProperty("Optional project id or project space id."))
                        .put("project_index", intProperty("Optional project index."))
                        .put("conversation_id", stringProperty("Optional local conversation id."))
                        .put("conversation_index", intProperty("Optional local conversation index."))
                        .put("conversation_title", stringProperty("Optional title when creating a conversation."))
                        .put("title", stringProperty("Optional alias for conversation_title."))
                        .put("text", stringProperty("Text for set_input_text."))
                        .put("message", stringProperty("Message for send_project_message."))
                        .put("control_id", stringProperty("Stable ChatGPT semantic control id returned by ui_state."))
                        .put("option_id", stringProperty("Stable ChatGPT model or tool option id returned by chatgpt_get_navigation."))
                        .put("feature_id", stringProperty("Stable ChatGPT feature id returned by chatgpt_get_navigation."))
                        .put("query", stringProperty("Optional label/title query for chatgpt_find_controls or chatgpt_get_conversations."))
                        .put("semantic", stringProperty("Optional semantic filter for chatgpt_find_controls."))
                        .put("region", stringProperty("Optional region filter for chatgpt_find_controls."))
                        .put("context_id", stringProperty("Optional message or conversation context id for chatgpt_find_controls."))
                        .put("conversation_path", stringProperty("Official ChatGPT conversation path such as /c/example."))
                        .put("view_mode", stringProperty("ChatGPT view mode: native, official, or login."))
                        .put("section", stringProperty("ChatGPT composer section model or tools; also filters chatgpt_get_navigation."))
                        .put("offset", intProperty("Zero-based page offset for ChatGPT control and conversation queries."))
                        .put("limit", intProperty("Page size for ChatGPT control and conversation queries."))
                        .put("message_offset", intProperty("Zero-based message offset for chatgpt_get_context."))
                        .put("message_limit", intProperty("Page size from 1 to 40 for chatgpt_get_context."))
                        .put("new_conversation", booleanProperty("Create a new conversation before send_project_message. Defaults to false."))
                        .put("main_thread_timeout_ms", intProperty("How long native MCP waits for the APK main thread, 1000-60000. Defaults to 15000.")),
                    required = JSONArray().put("action")
                )
            )
            .put(
                tool(
                    name = "update_status",
                    title = "APK Update Status",
                    description = "Compare the installed APK version with the server version.json.",
                    properties = JSONObject()
                        .put("server_url", stringProperty("Optional version.json URL.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "task_status",
                    title = "Task Status",
                    description = "Return the active or most recent task status with timing milestones and last message preview.",
                    properties = JSONObject()
                        .put("trace_id", stringProperty("Optional trace id. Defaults to current pending task or latest traced task."))
                        .put("include_events", booleanProperty("Include recent trace events for the selected trace. Defaults to false."))
                        .put("event_limit", intProperty("Maximum trace events to include, 1-300. Defaults to 80.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "task_control",
                    title = "Task Control",
                    description = "Inspect or pause the active phone task from inside the APK process.",
                    properties = JSONObject()
                        .put("action", stringProperty("One of status or pause. Defaults to status.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "task_events",
                    title = "Queued Task Events",
                    description = "Return raw task events queued while the UI was in the background, with an option to clear them after reading.",
                    properties = JSONObject()
                        .put("limit", intProperty("Maximum queued events to return, 1-120. Defaults to 40."))
                        .put("clear", booleanProperty("Clear queued events after reading. Defaults to false.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "logcat_recent",
                    title = "Recent Logcat",
                    description = "Return filtered recent logcat lines visible to the APK process.",
                    properties = JSONObject()
                        .put("line_count", intProperty("Raw logcat lines to scan, 20-1000. Defaults to 300."))
                        .put("pattern", stringProperty("Regex filter. Defaults to Elon/AndroidRuntime crash tags.")),
                    required = JSONArray()
                )
            )
            .put(
                tool(
                    name = "chat_send",
                    title = "Send Chat",
                    description = "Queue a chat request on the phone and seed the native conversation with the user message, collapsible process layer, and final-reply path.",
                    properties = JSONObject()
                        .put("message", stringProperty("Chat message to send from the phone."))
                        .put("project_id", stringProperty("Optional project id. Defaults to the active project."))
                        .put("project_title", stringProperty("Optional project title."))
                        .put("conversation_id", stringProperty("Optional conversation id for native CLI session continuity."))
                        .put("conversation_title", stringProperty("Optional conversation title."))
                        .put("agent", stringProperty("Optional backend agent id, such as codex_cli."))
                        .put("runtimeRoute", stringProperty("Optional runtime route, such as route_a for local Codex."))
                        .put("execution_mode", stringProperty("Optional execution mode, such as execute or plan."))
                        .put("plan_mode", booleanProperty("Optional plan-mode flag passed to the backend project task."))
                        .put("local_node_id", stringProperty("Optional target PC node id for this project request."))
                        .put("local_workspace_path", stringProperty("Optional workspace path on the target PC node."))
                        .put("trace_id", stringProperty("Optional caller-provided trace id."))
                        .put("is_development", booleanProperty("Whether this should be treated as a development task."))
                        .put("show_in_ui", booleanProperty("Open the seeded native project conversation in the APK when MainActivity is bound. Defaults to true."))
                        .put("force", booleanProperty("Override an active phone task. Defaults to false."))
                        .put("start_ack_timeout_ms", intProperty("How long MCP waits for TaskWorkService to acknowledge startup, 0-10000. Defaults to 1800.")),
                    required = JSONArray().put("message")
                )
            )
            .put(
                tool(
                    name = "chat_probe",
                    title = "Chat Probe",
                    description = "Send a probe chat through the phone, wait for a milestone, then return task status, latency report, and optional diagnostic bundle in one call.",
                    properties = JSONObject()
                        .put("message", stringProperty("Probe chat message to send from the phone. Defaults to an ordinary chat unless is_development=true."))
                        .put("project_id", stringProperty("Optional project id. Defaults to the active project."))
                        .put("project_title", stringProperty("Optional project title."))
                        .put("conversation_id", stringProperty("Optional conversation id for native CLI session continuity."))
                        .put("conversation_title", stringProperty("Optional conversation title."))
                        .put("agent", stringProperty("Optional backend agent id, such as codex_cli."))
                        .put("runtimeRoute", stringProperty("Optional runtime route, such as route_a for local Codex."))
                        .put("execution_mode", stringProperty("Optional execution mode, such as execute or plan."))
                        .put("plan_mode", booleanProperty("Optional plan-mode flag passed to the backend project task."))
                        .put("local_node_id", stringProperty("Optional target PC node id for this project request."))
                        .put("local_workspace_path", stringProperty("Optional workspace path on the target PC node."))
                        .put("trace_id", stringProperty("Optional caller-provided trace id."))
                        .put("is_development", booleanProperty("Whether this probe should run as a development task. Defaults to false."))
                        .put("force", booleanProperty("Override an active phone task. Defaults to false."))
                        .put("start_ack_timeout_ms", intProperty("How long MCP waits for TaskWorkService to acknowledge startup, 0-10000. Defaults to 1800."))
                        .put("wait_for", stringProperty("One of queued, task_start, payload_sent, first_server_event, first_reply, finish. Defaults to first_reply."))
                        .put("wait_timeout_ms", intProperty("How long to wait for the milestone, 0-120000. Defaults to 25000."))
                        .put("poll_interval_ms", intProperty("Trace polling interval, 100-2000. Defaults to 350."))
                        .put("timeline_limit", intProperty("Maximum latency timeline events to return, 1-300. Defaults to 80."))
                        .put("include_diagnostic_bundle", booleanProperty("Include diagnostic_bundle after waiting. Defaults to true."))
                        .put("include_logcat", booleanProperty("Include logcat in the nested diagnostic bundle. Defaults to false."))
                        .put("include_network_check", booleanProperty("Include network_check in the nested diagnostic bundle. Defaults to true."))
                        .put("include_server_trace", booleanProperty("Include server_trace in the probe result and nested diagnostic bundle. Defaults to true."))
                        .put("server_trace_limit", intProperty("Maximum backend trace events to return, 1-300. Defaults to 120."))
                        .put("server_url", stringProperty("Optional backend base URL. Defaults to http://43.139.149.158:8080.")),
                    required = JSONArray().put("message")
                )
            )
    )
}

private fun tool(
    name: String,
    title: String,
    description: String,
    properties: JSONObject,
    required: JSONArray
): JSONObject {
    val mergedProperties = JSONObject()
        .put("auth_token", stringProperty("MCP debug token from GET /health or logcat tag ElonMcpServer."))
    properties.keys().forEach { key -> mergedProperties.put(key, properties.get(key)) }
    val mergedRequired = JSONArray().put("auth_token")
    for (index in 0 until required.length()) mergedRequired.put(required.get(index))
    return JSONObject()
        .put("name", name)
        .put("title", title)
        .put("description", description)
        .put(
            "inputSchema",
            JSONObject()
                .put("type", "object")
                .put("properties", mergedProperties)
                .put("required", mergedRequired)
                .put("additionalProperties", false)
        )
}

private fun stringProperty(description: String) =
    JSONObject().put("type", "string").put("description", description)

private fun intProperty(description: String) =
    JSONObject().put("type", "integer").put("description", description)

private fun booleanProperty(description: String) =
    JSONObject().put("type", "boolean").put("description", description)

private fun arrayProperty(description: String) =
    JSONObject().put("type", "array").put("description", description).put("items", JSONObject().put("type", "string"))
