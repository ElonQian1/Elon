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

  function createFunctionToolProvider(options) {
    options = options || {};
    var tools = options.tools || {};
    var collectContext = options.collectContext;
    return {
      async getToolManifest() {
        return {
          schema: "elon.route_c.tool_manifest.v0",
          tools: Object.keys(tools).map(function (name) {
            var entry = tools[name];
            return {
              name: normalizeToolName(name),
              description: entry && entry.description ? entry.description : undefined,
            };
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
      async getToolManifest() {
        return {
          schema: "elon.route_c.tool_manifest.v0",
          transport: "http",
          baseUrl: baseUrl,
          tools: Object.keys(tools).map(function (name) {
            var spec = tools[name] || {};
            return {
              name: normalizeToolName(name),
              description: spec.description,
              method: spec.method || "POST",
              path: spec.path,
            };
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
    }

    async ask(message, options) {
      options = options || {};
      var toolProvider = options.toolProvider || this.toolProvider;
      var toolManifest =
        options.toolManifest ||
        (toolProvider && toolProvider.getToolManifest
          ? await toolProvider.getToolManifest()
          : { schema: "elon.route_c.tool_manifest.v0", tools: [] });
      var localContext =
        options.localContext ||
        (toolProvider && toolProvider.collectContext
          ? await toolProvider.collectContext({ message: message })
          : {});
      var payload = {
        conversation_id: options.conversationId || this.conversationId,
        message: message,
        history: options.history || [],
        client: Object.assign({}, this.defaultClient, options.client || {}),
        local_context: localContext,
        tool_manifest: normalizeManifest(toolManifest),
        tool_results: options.toolResults || [],
        max_actions: options.maxActions || this.maxActions,
        agent: options.agent || this.agent,
        sdk: {
          name: "elon-route-c-sdk-js",
          version: SDK_VERSION,
          runtime: typeof window === "undefined" ? "node_or_worker" : "browser",
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
    normalizeToolName: normalizeToolName,
  };
});
