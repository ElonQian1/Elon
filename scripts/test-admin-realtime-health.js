const assert = require('assert');
const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const adminHtmlPath = path.join(repoRoot, 'server', 'src', 'assets', 'admin.html');

class FakeElement {
  constructor(id) {
    this.id = id;
    this.innerHTML = '';
    this.textContent = '';
    this.value = '';
    this.className = '';
    this.dataset = {};
    this.style = {};
    this.type = 'text';
    this.checked = false;
    this.disabled = false;
    this.classList = {
      values: new Set(),
      add: (...names) => names.forEach((name) => this.classList.values.add(name)),
      remove: (...names) => names.forEach((name) => this.classList.values.delete(name)),
      contains: (name) => this.classList.values.has(name),
    };
  }

  focus() {}
}

function createFakeDocument() {
  const elements = new Map();
  return {
    elements,
    getElementById(id) {
      if (!elements.has(id)) elements.set(id, new FakeElement(id));
      return elements.get(id);
    },
    querySelectorAll() {
      return [];
    },
    addEventListener() {},
  };
}

function loadAdminScript(document, fetchImpl) {
  const html = fs.readFileSync(adminHtmlPath, 'utf8');
  const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/gi)]
    .map((match) => match[1])
    .join('\n');

  const localStorage = {
    values: new Map([['elon_admin_token', 'smoke-admin-token']]),
    getItem(key) {
      return this.values.get(key) || null;
    },
    setItem(key, value) {
      this.values.set(key, String(value));
    },
  };

  const window = {
    addEventListener() {},
  };

  return new Function(
    'window',
    'document',
    'localStorage',
    'setTimeout',
    'clearTimeout',
    'fetch',
    'console',
    'prompt',
    'confirm',
    `${scripts}
return {
  loadRealtimeHealth,
  renderRealtimeHealth,
  renderRealtimeAlerts,
  realtimeReasonColor,
  realtimeChannelColor,
};`,
  )(
    window,
    document,
    localStorage,
    () => 0,
    () => {},
    fetchImpl,
    console,
    () => null,
    () => true,
  );
}

async function main() {
  const document = createFakeDocument();
  const requests = [];
  const metricsPayload = {
    metrics: [
      { channel: 'peer_relay', close_reason: 'peer_write_error', count: 99 },
    ],
    windows: {
      last_1h: [
        { channel: 'global_app', close_reason: 'read_error', count: 3 },
        { channel: 'homecli_agent', close_reason: 'reader_timeout', count: 2 },
      ],
      last_24h: [
        { channel: 'voice_transcribe', close_reason: 'client_control_close', count: 7 },
      ],
      all_time: [
        { channel: 'peer_relay', close_reason: 'peer_write_error', count: 99 },
      ],
      process: [
        { channel: 'project_ws', close_reason: 'write_failed', count: 1 },
      ],
    },
    alerts: [
      {
        fingerprint: 'realtime:read-errors-last-hour',
        severity: 'critical',
        title: 'Realtime WebSocket read errors elevated',
        detail: 'unsafe <script>alert(1)</script> detail',
        metric_value: 3,
        updated_at: '2026-07-16T12:00:00Z',
      },
    ],
  };
  const diagnosticsPayload = {
    version: '2026-07-16',
    channels: [
      {
        id: 'global_app',
        business_boundary: 'Global app websocket for online presence and common pushes',
        entry_modules: ['server/src/global_ws.rs'],
        close_reason_source: 'WsCloseReason',
        metric_variant: 'RealtimeChannel::GlobalApp',
        sync_targets: ['docs/realtime-operations-runbook.md'],
      },
      {
        id: 'homecli_agent',
        business_boundary: 'HomeCLI/PC agent reverse websocket for PC CLI dispatch',
        entry_modules: ['server/src/homecli_agent/agent_session.rs'],
        close_reason_source: 'AgentSessionCloseReason',
        metric_variant: 'RealtimeChannel::HomecliAgent',
        sync_targets: ['docs/realtime-operations-runbook.md'],
      },
    ],
    close_reasons: [
      {
        id: 'read_error',
        source: 'WsCloseReason',
        category: 'read_error',
        alert_bucket: 'read_error',
        meaning: 'Server failed to read a websocket frame.',
        first_check: 'Check client network quality and reverse proxy behavior.',
      },
      {
        id: 'reader_timeout',
        source: 'AgentSessionCloseReason',
        category: 'timeout',
        alert_bucket: 'timeout',
        meaning: 'HomeCLI agent reader timed out.',
        first_check: 'Check PC sleep, node false-online state, and heartbeat loss.',
      },
      {
        id: 'peer_write_error',
        source: 'PeerWsCloseReason',
        category: 'write_failure',
        alert_bucket: 'write_failure',
        meaning: 'Server failed to write to a peer relay seeder websocket.',
        first_check: 'Check relay command delivery and seeder disconnects.',
      },
    ],
    change_rules: ['Keep record_close_with_store and diagnostics catalog in sync.'],
  };

  async function fetchImpl(requestPath, options = {}) {
    requests.push({ requestPath, options });
    return {
      ok: true,
      status: 200,
      async json() {
        if (requestPath === '/api/admin/realtime/diagnostics') return diagnosticsPayload;
        return metricsPayload;
      },
    };
  }

  const admin = loadAdminScript(document, fetchImpl);
  document.getElementById('realtimeWindow').value = 'last_1h';

  await admin.loadRealtimeHealth();

  assert.strictEqual(requests.length, 2, 'Realtime smoke should make metrics and diagnostics requests');
  assert.strictEqual(
    requests[0].requestPath,
    '/api/admin/realtime/close-metrics',
    'Realtime panel should call the close metrics endpoint',
  );
  assert.strictEqual(
    requests[1].requestPath,
    '/api/admin/realtime/diagnostics',
    'Realtime panel should call the diagnostics endpoint',
  );
  assert.strictEqual(requests[0].options.method, 'GET');
  assert.strictEqual(
    requests[0].options.headers.Authorization,
    'Bearer smoke-admin-token',
    'Realtime request should carry the admin token',
  );
  assert.strictEqual(
    requests[1].options.headers.Authorization,
    'Bearer smoke-admin-token',
    'Realtime diagnostics request should carry the admin token',
  );

  const cards = document.getElementById('realtimeSummaryCards').innerHTML;
  const tbody = document.getElementById('realtimeCloseMetricsBody').innerHTML;
  const closeSubtitle = document.getElementById('realtimeCloseSubtitle').textContent;
  const alerts = document.getElementById('realtimeAlertsList').innerHTML;
  const alertsSubtitle = document.getElementById('realtimeAlertsSubtitle').textContent;

  assert.ok(cards.includes('total_closes'), 'summary cards should include total close count');
  assert.ok(cards.includes('>5<'), 'last_1h close count should total selected rows only');
  assert.ok(cards.includes('global_app'), 'summary cards should include global_app totals');
  assert.ok(cards.includes('homecli_agent'), 'summary cards should include homecli_agent totals');
  assert.ok(!cards.includes('peer_relay'), 'summary cards should not render all_time rows for last_1h');

  assert.strictEqual(
    closeSubtitle,
    'last_1h / 2 reason rows / 2 channels',
    'close subtitle should describe the selected window',
  );
  assert.ok(tbody.includes('read_error'), 'detail table should render read_error rows');
  assert.ok(tbody.includes('reader_timeout'), 'detail table should render reader_timeout rows');
  assert.ok(tbody.includes('read_error / read_error'), 'detail table should render read_error diagnostics category and bucket');
  assert.ok(tbody.includes('timeout / timeout'), 'detail table should render timeout diagnostics category and bucket');
  assert.ok(tbody.includes('Check client network quality'), 'detail table should render first_check diagnostics');
  assert.ok(tbody.includes('Check PC sleep'), 'detail table should render timeout first_check diagnostics');
  assert.ok(!tbody.includes('peer_write_error'), 'detail table should not render unselected windows');

  assert.strictEqual(alertsSubtitle, '1 open', 'alert subtitle should show open alert count');
  assert.ok(alerts.includes('critical'), 'alerts panel should render severity');
  assert.ok(alerts.includes('Realtime WebSocket read errors elevated'), 'alerts panel should render title');
  assert.ok(alerts.includes('realtime:read-errors-last-hour'), 'alerts panel should render fingerprint');
  assert.ok(alerts.includes('bucket: read_error'), 'alerts panel should render diagnostics bucket');
  assert.ok(alerts.includes('first check: Check client network quality'), 'alerts panel should render diagnostics first check');
  assert.ok(alerts.includes('&lt;script&gt;alert(1)&lt;/script&gt;'), 'alert detail should be escaped');
  assert.ok(!alerts.includes('<script>alert(1)</script>'), 'alert detail must not inject raw HTML');

  assert.strictEqual(admin.realtimeReasonColor('read_error'), 'var(--danger)');
  assert.strictEqual(admin.realtimeReasonColor('reader_timeout'), 'var(--warn)');
  assert.strictEqual(admin.realtimeChannelColor('voice_transcribe'), '#38bdf8');

  console.log('admin realtime health smoke passed');
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
