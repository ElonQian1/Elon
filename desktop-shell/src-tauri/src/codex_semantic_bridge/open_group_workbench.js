(function () {
  const current = new URL(window.location.href);
  const boot = window.__ELON_PC_BOOTSTRAP__ || {};
  const loopback = url => ['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname);
  const local = boot.mode === 'local' || loopback(current);
  const cloud = new URL(local ? (boot.cloudBaseUrl || 'http://43.139.149.158:8080') : current.origin);
  if (!['http:', 'https:'].includes(cloud.protocol) || cloud.username || cloud.password || cloud.search || cloud.hash) {
    throw new Error('invalid_workbench_origin');
  }
  const node = new URL(boot.localNodeBaseUrl || (loopback(current) ? current.origin : 'http://127.0.0.1:7799'));
  if (node.protocol !== 'http:' || !loopback(node) || node.username || node.password || node.search || node.hash) {
    throw new Error('invalid_local_node_origin');
  }
  const target = new URL('/pc/friends', cloud.origin);
  target.searchParams.set('node_admin', node.origin + '/');
  // Navigation happens only after the semantic action can publish its receipt.
  window.setTimeout(() => window.location.assign(target.href), 700);
})();
