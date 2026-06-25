(function (root, factory) {
  if (typeof module === "object" && module.exports) {
    module.exports = factory();
  } else {
    root.ElonRouteCSDK = factory();
  }
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";

  var SDK_VERSION = "0.1.0";

  function nowIso() {
    return new Date().toISOString();
  }

  function normalizeToolName(name) {
    return String(name || "").trim().toLowerCase().replace(/-/g, "_");
  }

  function normalizeRuntimeRoute(route) {
    var normalized = String(route || "route_c")
      .trim()
      .toLowerCase()
      .replace(/[\s-]+/g, "_");
    if (
      normalized === "a" ||
      normalized === "route_a" ||
      normalized === "local_cli" ||
      normalized === "local_ai_cli" ||
      normalized === "codex_cli" ||
      normalized === "copilot_cli" ||
      normalized === "claude_cli"
    ) {
      return "route_a";
    }
    if (
      normalized === "b" ||
      normalized === "route_b" ||
      normalized === "byok" ||
      normalized === "bring_your_own_key" ||
      normalized === "local_api_key" ||
      normalized === "user_api_key"
    ) {
      return "route_b";
    }
    return "route_c";
  }

  function defaultFetch() {
    if (typeof fetch !== "function") {
      throw new Error("ElonRouteCSDK requires fetch or options.fetchImpl");
    }
    return fetch.bind(globalThis);
  }

  function compactError(error) {
    if (!error) return "unknown_error";
    if (error && error.message) return String(error.message);
    return String(error);
  }

  function normalizeManifest(manifest) {
    if (!manifest) return { schema: "elon.route_c.tool_manifest.v0", tools: [] };
    if (Array.isArray(manifest)) {
      return {
        schema: "elon.route_c.tool_manifest.v0",
        tools: manifest.map(function (tool) {
          if (typeof tool === "string") return { name: normalizeToolName(tool) };
          return Object.assign({}, tool, {
            name: normalizeToolName(tool.name || tool.id || tool.tool),
          });
        }),
      };
    }
    return manifest;
  }

  function responseError(response, bodyText) {
    var err = new Error("Route C SDK request failed: HTTP " + response.status);
    err.status = response.status;
    err.body = bodyText;
    return err;
  }

  function makeToolResult(action, startedAt, payload) {
    return Object.assign(
      {
        schema: "elon.route_c.tool_result.v0",
        tool_call_id: action.id || null,
        tool: action.tool,
        args: action.args || {},
        generated_at: nowIso(),
        duration_ms: Date.now() - startedAt,
      },
      payload
    );
  }

  function manifestRuntimePermission(manifest) {
    if (!manifest || typeof manifest !== "object") return null;
    var direct = manifest.runtime_permission || manifest.runtimePermission || manifest.permission;
    if (direct === "danger_full_access") return "danger_full_access";
    var tools = Array.isArray(manifest.tools) ? manifest.tools : [];
    for (var i = 0; i < tools.length; i += 1) {
      var tool = tools[i] || {};
      var permission = tool.runtime_permission || tool.runtimePermission || tool.permission;
      if (permission === "danger_full_access" || tool.dangerous === true) {
        return "danger_full_access";
      }
    }
    return null;
  }

  function toolManifestEntry(name, entry) {
    var manifest = entry && entry.manifest && typeof entry.manifest === "object" ? entry.manifest : {};
    var spec = Object.assign({}, manifest, { name: normalizeToolName(name) });
    var description = entry && entry.description ? entry.description : manifest.description;
    if (description) spec.description = description;
    return spec;
  }

  function createFunctionToolProvider(options) {
    options = options || {};
    var tools = options.tools || {};
    var collectContext = options.collectContext;
    return {
      runtimePermission: options.runtimePermission || options.runtime_permission || null,
      async getToolManifest() {
        return {
          schema: "elon.route_c.tool_manifest.v0",
          tools: Object.keys(tools).map(function (name) {
            var entry = tools[name];
            return toolManifestEntry(name, entry);
          }),
        };
      },
      async collectContext(input) {
        if (typeof collectContext === "function") {
          return collectContext(input);
        }
        return {};
      },
      async executeTool(name, args, action) {
        var normalized = normalizeToolName(name);
        var entry = tools[normalized] || tools[name];
        if (!entry) throw new Error("local tool not found: " + name);
        if (typeof entry === "function") return entry(args || {}, action || {});
        if (typeof entry.handler === "function") return entry.handler(args || {}, action || {});
        throw new Error("local tool has no handler: " + name);
      },
    };
  }

  function createHttpToolProvider(options) {
    options = options || {};
    var baseUrl = String(options.baseUrl || "").replace(/\/+$/, "");
    var fetchImpl = options.fetchImpl || defaultFetch();
    var collectContext = options.collectContext;
    var tools = options.tools || {};
    return {
      runtimePermission: options.runtimePermission || options.runtime_permission || null,
      async getToolManifest() {
        return {
          schema: "elon.route_c.tool_manifest.v0",
          transport: "http",
          baseUrl: baseUrl,
          tools: Object.keys(tools).map(function (name) {
            var spec = tools[name] || {};
            var manifest = spec.manifest && typeof spec.manifest === "object" ? spec.manifest : {};
            return Object.assign({}, manifest, {
              name: normalizeToolName(name),
              description: spec.description,
              method: spec.method || "POST",
              path: spec.path,
            });
          }),
        };
      },
      async collectContext(input) {
        if (typeof collectContext === "function") {
          return collectContext(input);
        }
        return {};
      },
      async executeTool(name, args, action) {
        var normalized = normalizeToolName(name);
        var spec = tools[normalized] || tools[name];
        if (!spec) throw new Error("local http tool not found: " + name);
        var method = String(spec.method || "POST").toUpperCase();
        var path = spec.path || "/" + encodeURIComponent(normalized);
        var url = /^https?:\/\//i.test(path) ? path : baseUrl + path;
        var init = { method: method, headers: Object.assign({}, spec.headers || {}) };
        if (method !== "GET" && method !== "HEAD") {
          init.headers["content-type"] = init.headers["content-type"] || "application/json";
          init.body = JSON.stringify(spec.body ? spec.body(args || {}, action || {}) : args || {});
        }
        var response = await fetchImpl(url, init);
        var text = await response.text();
        if (!response.ok) throw responseError(response, text);
        if (!text.trim()) return null;
        try {
          return JSON.parse(text);
        } catch (_) {
          return text;
        }
      },
    };
  }

  function registerProjectTool(tools, name, description, handler, manifest) {
    if (typeof handler !== "function") return false;
    tools[normalizeToolName(name)] = {
      description: description,
      manifest: Object.assign({}, manifest || {}),
      handler: function (args, action) {
        return handler(args || {}, action || {});
      },
    };
    return true;
  }

  function createProjectAiToolProvider(options) {
    options = options || {};
    var runtimeRoute = normalizeRuntimeRoute(
      options.runtimeRoute || options.runtime_route || options.route || "route_c"
    );
    var tools = {};
    var businessTools = options.tools || options.businessTools || options.business_tools || {};
    Object.keys(businessTools).forEach(function (name) {
      tools[normalizeToolName(name)] = businessTools[name];
    });

    var localCli = options.localCli || options.local_cli || options.cli || {};
    var executeCommand =
      options.executeCommand ||
      options.runCommand ||
      localCli.executeCommand ||
      localCli.runCommand;
    var readFile = options.readFile || localCli.readFile;
    var writeFile = options.writeFile || localCli.writeFile;
    var listDir = options.listDir || localCli.listDir;
    var hasLocalCli = false;
    hasLocalCli =
      registerProjectTool(
        tools,
        "run_command",
        "Run arbitrary local cmd, powershell, pwsh, bash, sh, or executable commands.",
        executeCommand,
        { permission: "danger_full_access", dangerous: true }
      ) || hasLocalCli;
    hasLocalCli =
      registerProjectTool(
        tools,
        "read_file",
        "Read a local text file by relative or absolute path.",
        readFile,
        { permission: "danger_full_access", dangerous: true }
      ) || hasLocalCli;
    hasLocalCli =
      registerProjectTool(
        tools,
        "write_file",
        "Create or replace a local text file by relative or absolute path.",
        writeFile,
        { permission: "danger_full_access", dangerous: true }
      ) || hasLocalCli;
    hasLocalCli =
      registerProjectTool(
        tools,
        "list_dir",
        "List a local directory by relative or absolute path.",
        listDir,
        { permission: "danger_full_access", dangerous: true }
      ) || hasLocalCli;

    var remoteSource = options.remoteSource || options.remote_source || {};
    var remoteSearch =
      options.remoteSourceSearch ||
      options.remote_source_search ||
      remoteSource.search ||
      remoteSource.searchSource;
    var remoteReadFile =
      options.remoteSourceReadFile ||
      options.remote_source_read_file ||
      remoteSource.readFile ||
      remoteSource.read_file;
    var remoteAsk =
      options.remoteSourceAsk ||
      options.remote_source_ask ||
      remoteSource.ask ||
      remoteSource.askSource;
    var hasRemoteSource = false;
    hasRemoteSource =
      registerProjectTool(
        tools,
        "remote_source_search",
        "Ask a remote project source node to search source files, symbols, docs, or commits.",
        remoteSearch,
        { category: "remote_source", dangerous: false }
      ) || hasRemoteSource;
    hasRemoteSource =
      registerProjectTool(
        tools,
        "remote_source_read_file",
        "Ask a remote project source node to read a specific source file or snippet.",
        remoteReadFile,
        { category: "remote_source", dangerous: false }
      ) || hasRemoteSource;
    hasRemoteSource =
      registerProjectTool(
        tools,
        "remote_source_ask",
        "Ask a remote project source node to inspect source code and answer a focused question.",
        remoteAsk,
        { category: "remote_source", dangerous: false }
      ) || hasRemoteSource;

    var feedback = options.feedback || options.issue || options.demand || {};
    var createFeedbackPost =
      options.createFeedbackPost ||
      options.create_feedback_post ||
      feedback.createPost ||
      feedback.createFeedbackPost ||
      feedback.post;
    var hasFeedback = registerProjectTool(
      tools,
      "create_feedback_post",
      "Create a child project demand or feedback-channel post with user evidence and source findings.",
      createFeedbackPost,
      { category: "feedback", dangerous: false }
    );

    var runtimePermission =
      options.runtimePermission ||
      options.runtime_permission ||
      (hasLocalCli && options.dangerFullAccess !== false ? "danger_full_access" : null);
    var collectContext = options.collectContext;
    var provider = createFunctionToolProvider({
      runtimePermission: runtimePermission,
      tools: tools,
      collectContext: async function (input) {
        var context = {
          project_ai: {
            schema: "elon.project_ai_sdk.mvp.v0",
            runtime_route: runtimeRoute,
            source_project: options.sourceProject || options.source_project || options.appId || null,
            local_cli_enabled: hasLocalCli,
            remote_source_enabled: hasRemoteSource,
            feedback_enabled: Boolean(hasFeedback),
          },
        };
        if (runtimePermission) context.runtime_permission = runtimePermission;
        if (typeof collectContext === "function") {
          var extra = await collectContext(
            Object.assign({ runtime_route: runtimeRoute }, input || {})
          );
          if (extra && typeof extra === "object" && !Array.isArray(extra)) {
            Object.assign(context, extra);
          } else if (extra != null) {
            context.extra_context = extra;
          }
        }
        return context;
      },
    });
    provider.runtimeRoute = runtimeRoute;
    provider.runtime_route = runtimeRoute;
    return provider;
  }

  function createDangerFullAccessToolProvider(options) {
    options = options || {};
    var executeCommand = options.executeCommand || options.runCommand;
    var readFile = options.readFile;
    var writeFile = options.writeFile;
    var listDir = options.listDir;
    var collectContext = options.collectContext;
    if (typeof executeCommand !== "function") {
      throw new Error("createDangerFullAccessToolProvider requires options.executeCommand");
    }
    if (typeof readFile !== "function") {
      throw new Error("createDangerFullAccessToolProvider requires options.readFile");
    }
    if (typeof writeFile !== "function") {
      throw new Error("createDangerFullAccessToolProvider requires options.writeFile");
    }
    var tools = {
      run_command: {
        description:
          "Run arbitrary cmd, powershell, pwsh, bash, sh, or direct executable commands on the user's local machine.",
        manifest: { permission: "danger_full_access", dangerous: true },
        handler: function (args, action) {
          return executeCommand(args || {}, action || {});
        },
      },
      read_file: {
        description: "Read a local text file by relative or absolute path.",
        manifest: { permission: "danger_full_access", dangerous: true },
        handler: function (args, action) {
          return readFile(args || {}, action || {});
        },
      },
      write_file: {
        description: "Create or replace a local text file by relative or absolute path.",
        manifest: { permission: "danger_full_access", dangerous: true },
        handler: function (args, action) {
          return writeFile(args || {}, action || {});
        },
      },
    };
    if (typeof listDir === "function") {
      tools.list_dir = {
        description: "List a local directory by relative or absolute path.",
        manifest: { permission: "danger_full_access", dangerous: true },
        handler: function (args, action) {
          return listDir(args || {}, action || {});
        },
      };
    }
    return createFunctionToolProvider({
      runtimePermission: "danger_full_access",
      tools: tools,
      collectContext: function (input) {
        if (typeof collectContext === "function") {
          return collectContext(
            Object.assign({ runtime_permission: "danger_full_access" }, input || {})
          );
        }
        return { runtime_permission: "danger_full_access" };
      },
    });
  }

  class ElonRouteCClient {
    constructor(options) {
      options = options || {};
      this.appId = options.appId || "bb64a";
      this.serverBaseUrl = String(options.serverBaseUrl || "").replace(/\/+$/, "");
      this.endpoint =
        options.endpoint ||
        this.serverBaseUrl + "/api/external/apps/" + encodeURIComponent(this.appId) + "/route-c/chat";
      this.conversationId = options.conversationId || null;
      this.toolProvider = options.toolProvider || null;
      this.fetchImpl = options.fetchImpl || defaultFetch();
      this.maxToolRounds = options.maxToolRounds == null ? 4 : Number(options.maxToolRounds);
      this.maxActions = options.maxActions == null ? 3 : Number(options.maxActions);
      this.defaultClient = options.client || {};
      this.agent = options.agent || null;
      this.runtimePermission = options.runtimePermission || options.runtime_permission || null;
      this.runtimeRoute = normalizeRuntimeRoute(
        options.runtimeRoute || options.runtime_route || options.route || "route_c"
      );
    }

    async ask(message, options) {
      options = options || {};
      var toolProvider = options.toolProvider || this.toolProvider;
      var toolManifest =
        options.toolManifest ||
        (toolProvider && toolProvider.getToolManifest
          ? await toolProvider.getToolManifest()
          : { schema: "elon.route_c.tool_manifest.v0", tools: [] });
      var normalizedManifest = normalizeManifest(toolManifest);
      var runtimePermission =
        options.runtimePermission ||
        options.runtime_permission ||
        (toolProvider && (toolProvider.runtimePermission || toolProvider.runtime_permission)) ||
        this.runtimePermission ||
        manifestRuntimePermission(normalizedManifest);
      var runtimeRoute = normalizeRuntimeRoute(
        options.runtimeRoute ||
          options.runtime_route ||
          options.route ||
          (toolProvider && (toolProvider.runtimeRoute || toolProvider.runtime_route)) ||
          this.runtimeRoute
      );
      var localContext =
        options.localContext ||
        (toolProvider && toolProvider.collectContext
          ? await toolProvider.collectContext({ message: message, runtime_route: runtimeRoute })
          : {});
      var payload = {
        conversation_id: options.conversationId || this.conversationId,
        message: message,
        history: options.history || [],
        client: Object.assign({}, this.defaultClient, options.client || {}),
        local_context: localContext,
        tool_manifest: normalizedManifest,
        tool_results: options.toolResults || [],
        max_actions: options.maxActions || this.maxActions,
        agent: options.agent || this.agent,
        runtime_permission: runtimePermission,
        runtime_route: runtimeRoute,
        sdk: {
          name: "elon-route-c-sdk-js",
          version: SDK_VERSION,
          runtime: typeof window === "undefined" ? "node_or_worker" : "browser",
          runtime_permission: runtimePermission,
          runtime_route: runtimeRoute,
        },
      };
      var steps = [];
      var response = await this.chat(payload);
      if (response.conversation_id) this.conversationId = response.conversation_id;
      steps.push({ type: "model", response: response });

      var rounds = 0;
      while (
        toolProvider &&
        response &&
        Array.isArray(response.actions) &&
        response.actions.length &&
        rounds < this.maxToolRounds
      ) {
        rounds += 1;
        var toolResults = await this.executeActions(toolProvider, response.actions);
        steps.push({ type: "tools", results: toolResults });
        payload.tool_results = (payload.tool_results || []).concat(toolResults);
        payload.conversation_id = this.conversationId;
        response = await this.chat(payload);
        if (response.conversation_id) this.conversationId = response.conversation_id;
        steps.push({ type: "model", response: response });
      }

      return Object.assign({}, response, {
        conversation_id: this.conversationId || (response && response.conversation_id) || null,
        steps: steps,
        tool_rounds: rounds,
      });
    }

    async chat(payload) {
      var response = await this.fetchImpl(this.endpoint, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(payload),
      });
      var text = await response.text();
      if (!response.ok) throw responseError(response, text);
      return text ? JSON.parse(text) : {};
    }

    async executeActions(toolProvider, actions) {
      var results = [];
      for (var i = 0; i < actions.length; i += 1) {
        var action = actions[i] || {};
        var tool = normalizeToolName(action.tool);
        var startedAt = Date.now();
        try {
          var data = await toolProvider.executeTool(tool, action.args || {}, action);
          results.push(
            makeToolResult(action, startedAt, {
              success: true,
              status: "ok",
              data: data == null ? null : data,
              error: null,
            })
          );
        } catch (error) {
          results.push(
            makeToolResult(action, startedAt, {
              success: false,
              status: "error",
              data: null,
              error: compactError(error),
            })
          );
        }
      }
      return results;
    }
  }

  return {
    version: SDK_VERSION,
    ElonRouteCClient: ElonRouteCClient,
    createFunctionToolProvider: createFunctionToolProvider,
    createHttpToolProvider: createHttpToolProvider,
    createProjectAiToolProvider: createProjectAiToolProvider,
    createDangerFullAccessToolProvider: createDangerFullAccessToolProvider,
    normalizeToolName: normalizeToolName,
    normalizeRuntimeRoute: normalizeRuntimeRoute,
  };
});
