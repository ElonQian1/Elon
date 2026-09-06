"use strict";

(() => {
  const byId = (id) => document.getElementById(id);
  const actions = ["start", "stop", "clear", "export"];
  const controls = [...actions, "refresh"].map(byId);
  const errorMessages = {
    wrong_page: "请先打开 www.binance.com 的合约或合约网格页面，再启用观察。",
    not_started: "当前标签页尚未启用观察，请先启用。",
    no_samples: "暂时没有可导出的样本。零样本不代表没有网格。",
    operation_failed: "本次操作未完成，请更新状态后重试。",
    unsupported_browser: "当前浏览器不支持此观察方式，请使用 Chrome 106 或更新版本。",
  };
  const reasonLabels = {
    idle: ["尚未启用", "当前标签页还没有活动的观察会话。"],
    capturing: ["正在观察", "仅收集当前标签页后续请求的脱敏结构。"],
    stopped: ["观察已停止", "新的请求不会加入本轮摘要；已有摘要可手动导出。"],
    expired: ["观察已到期", "15 分钟时限已到；重新启用会清除前一轮摘要。"],
    navigated: ["页面已导航", "原页面的观察已停止，请在目标页面重新启用。"],
    cleared: ["摘要已清除", "当前标签页没有保留上一轮摘要。"],
    replaced: ["观察已替换", "另一轮观察已开始，旧一轮结果不会继续写入。"],
    unavailable: ["状态暂不可用", "请确认当前页面，然后更新状态。"],
  };
  let state = null;
  let busy = false;
  let closed = false;
  let lastPollAt = 0;
  const downloadUrls = new Set();

  function validState(value) {
    if (!value || typeof value !== "object" || Array.isArray(value)) return false;
    const expected = ["active", "records", "observations", "dropped", "startedAt", "expiresAt", "reason"];
    if (Object.keys(value).length !== expected.length || !expected.every((key) => Object.hasOwn(value, key))) return false;
    return typeof value.active === "boolean" && typeof value.reason === "string" && value.reason.length <= 80
      && ["records", "observations", "dropped", "startedAt", "expiresAt"].every((key) => Number.isSafeInteger(value[key]) && value[key] >= 0)
      && value.startedAt <= value.expiresAt && (!value.active || (value.startedAt > 0 && value.expiresAt > value.startedAt));
  }

  function feedback(message, kind = "info") {
    const element = byId("feedback");
    element.textContent = message;
    element.dataset.kind = kind;
    element.hidden = !message;
  }

  function render() {
    document.querySelector("main").setAttribute("aria-busy", String(busy));
    for (const button of controls) button.disabled = busy || closed;
    if (!state) {
      for (const action of actions) byId(action).disabled = true;
      return;
    }
    byId("start").disabled = busy || closed || state.active;
    byId("stop").disabled = busy || closed || !state.active;
    byId("export").disabled = busy || closed || state.records === 0;
    const labels = Object.hasOwn(reasonLabels, state.reason) ? reasonLabels[state.reason] : reasonLabels.unavailable;
    byId("status-label").textContent = state.active ? "正在观察" : labels[0];
    byId("status-detail").textContent = state.active ? reasonLabels.capturing[1] : labels[1];
    byId("status-dot").dataset.active = String(state.active);
    byId("record-count").textContent = String(state.records);
    byId("observation-count").textContent = String(state.observations);
    byId("dropped-count").textContent = String(state.dropped);
    const countdown = byId("countdown");
    countdown.hidden = !state.active;
    if (state.active) {
      const seconds = Math.max(0, Math.ceil((state.expiresAt - Date.now()) / 1000));
      countdown.textContent = seconds > 0 ? `剩余 ${Math.floor(seconds / 60)} 分 ${String(seconds % 60).padStart(2, "0")} 秒`
        : "观察时限已到，正在确认停止状态。";
    }
  }

  function downloadReport(report) {
    const fields = ["schema", "toolVersion", "provenance", "capturedAt", "exportedAt", "status", "coverage", "records"];
    if (!report || typeof report !== "object" || Array.isArray(report) || Object.keys(report).length !== fields.length
      || !fields.every((field) => Object.hasOwn(report, field)) || report.schema !== "binance-grid-observer-report.v1"
      || report.toolVersion !== "0.1.0" || report.provenance !== "untrusted_page_observation"
      || !validState(report.status) || !Array.isArray(report.records) || report.records.length > 128
      || report.records.length !== report.status.records || !report.coverage || typeof report.coverage !== "object"
      || report.coverage.contractVerified !== false || report.coverage.tradingEnabled !== false
      || report.coverage.requestValuesIncluded !== false || report.coverage.headersIncluded !== false
      || report.coverage.cookiesIncluded !== false || !Number.isSafeInteger(report.capturedAt) || report.capturedAt <= 0
      || !Number.isSafeInteger(report.exportedAt) || report.exportedAt < report.capturedAt) throw new Error("invalid_report");
    const text = JSON.stringify(report);
    if (typeof text !== "string" || new TextEncoder().encode(text).byteLength > 1_153_434) throw new Error("invalid_report");
    const url = URL.createObjectURL(new Blob([text], { type: "application/json;charset=utf-8" }));
    downloadUrls.add(url);
    const link = document.createElement("a");
    const time = new Date().toISOString().slice(0, 19).replace(/[-:]/g, "").replace("T", "-");
    link.href = url;
    link.download = `binance-grid-observation-${time}.json`;
    link.hidden = true;
    document.body.appendChild(link);
    try { link.click(); } finally {
      link.remove();
      setTimeout(() => { URL.revokeObjectURL(url); downloadUrls.delete(url); }, 1000);
    }
  }

  function markUnavailable() {
    state = null;
    byId("status-label").textContent = "无法读取状态";
    byId("status-detail").textContent = "请确认当前页面，再点击更新状态。";
    byId("status-dot").dataset.active = "false";
    byId("countdown").hidden = true;
    for (const id of ["record-count", "observation-count", "dropped-count"]) byId(id).textContent = "—";
  }

  async function send(action) {
    let timer;
    try {
      return await Promise.race([
        chrome.runtime.sendMessage({ type: "observer-ui", action }),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("operation_timeout")), 12_000); }),
      ]);
    } finally { clearTimeout(timer); }
  }

  async function perform(action, silent = false) {
    if (busy || closed) return;
    busy = true;
    if (!silent) feedback(action === "status" ? "正在更新状态…" : "正在处理，请稍候…");
    render();
    try {
      const response = await send(action);
      if (closed) return;
      if (!response || response.ok !== true) {
        const code = response && typeof response.error === "string" ? response.error : "operation_failed";
        feedback(Object.hasOwn(errorMessages, code) ? errorMessages[code] : errorMessages.operation_failed, "error");
        if (action === "status") markUnavailable();
        return;
      }
      if (!validState(response.state)) throw new Error("invalid_state");
      state = response.state;
      if (action === "export") {
        downloadReport(response.report);
        feedback("已发起摘要下载。导出内容仅用于研究，不代表交易或接口验收通过。");
      } else if (action === "start") feedback("已启用观察。现在可回到官网查看网格列表，页面不会被刷新。");
      else if (action === "stop") feedback("观察已停止，已有摘要仍可导出。");
      else if (action === "clear") feedback("摘要已清除。");
      else if (!silent) feedback("");
    } catch {
      if (!closed) {
        if (action === "status") markUnavailable();
        feedback(errorMessages.operation_failed, "error");
      }
    } finally {
      busy = false;
      lastPollAt = Date.now();
      if (!closed) render();
    }
  }

  for (const action of actions) byId(action).addEventListener("click", () => { void perform(action); });
  byId("refresh").addEventListener("click", () => { void perform("status"); });
  const ticker = setInterval(() => {
    if (closed) return;
    render();
    if (state && state.active && !busy && Date.now() - lastPollAt >= 3000) void perform("status", true);
  }, 1000);
  window.addEventListener("pagehide", () => {
    closed = true;
    clearInterval(ticker);
    for (const url of downloadUrls) URL.revokeObjectURL(url);
    downloadUrls.clear();
  }, { once: true });
  void perform("status");
})();
