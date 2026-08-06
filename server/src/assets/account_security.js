(function () {
  'use strict';
  const TOKEN_KEY = 'lodex_token';
  const CLIENT_KEY = 'elon_account_security_client_instance_id';
  const token = () => localStorage.getItem(TOKEN_KEY) || localStorage.getItem('elon_token') || '';
  const requestId = (action) => 'web:' + action + ':' + randomId();
  function randomId() {
    return crypto.randomUUID ? crypto.randomUUID() : Date.now() + '-' + Math.random().toString(16).slice(2);
  }
  function clientInstanceId() {
    let value = localStorage.getItem(CLIENT_KEY);
    if (!value) {
      value = 'web:' + randomId();
      localStorage.setItem(CLIENT_KEY, value);
    }
    return value;
  }
  async function api(path, options) {
    const headers = Object.assign(
      { 'Content-Type': 'application/json' },
      token() ? { Authorization: 'Bearer ' + token() } : {},
      options && options.headers
    );
    const response = await fetch(path, Object.assign({}, options, { headers }));
    const body = response.status === 204 ? {} : await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(body.error || body.message || '请求失败 (' + response.status + ')');
    return body;
  }
  function setStatus(node, message, error) {
    if (!node) return;
    node.textContent = message || '';
    node.classList.toggle('error', !!error);
  }

  function installLoginRecovery() {
    const toggle = document.getElementById('accountRecoveryToggle');
    const panel = document.getElementById('accountRecoveryPanel');
    const submit = document.getElementById('accountRecoverySubmit');
    const external = document.getElementById('accountExternalRecovery');
    const status = document.getElementById('accountRecoveryStatus');
    if (!toggle || !panel || !submit || !external || !status) return;
    toggle.addEventListener('click', () => panel.classList.toggle('active'));
    submit.addEventListener('click', async () => {
      const account = document.getElementById('accountRecoveryAccount').value.trim();
      const code = document.getElementById('accountRecoveryCode').value.trim();
      const password = document.getElementById('accountRecoveryPassword').value;
      const confirm = document.getElementById('accountRecoveryConfirm').value;
      if (!account || !code) return setStatus(status, '请输入账号和离线恢复码', true);
      if (password.length < 8) return setStatus(status, '新密码至少 8 位', true);
      if (password !== confirm) return setStatus(status, '两次输入的新密码不一致', true);
      submit.disabled = true;
      setStatus(status, '正在重置密码…');
      try {
        await api('/api/auth/password/recover', {
          method: 'POST',
          body: JSON.stringify({
            account, recovery_code: code, new_password: password,
            request_id: requestId('recover'), client_instance_id: clientInstanceId(), confirm: true
          })
        });
        document.getElementById('accountInput').value = account;
        setStatus(status, '密码已重置，所有旧会话已撤销。请使用新密码登录。');
      } catch (error) { setStatus(status, error.message || '重置失败', true); }
      submit.disabled = false;
    });
    external.addEventListener('click', async () => {
      const account = document.getElementById('accountRecoveryAccount').value.trim();
      if (!account) return setStatus(status, '请先输入账号', true);
      try {
        const result = await api('/api/auth/password/recovery/start', {
          method: 'POST', body: JSON.stringify({ account, client_instance_id: clientInstanceId() })
        });
        setStatus(status, result.message || '邮件或短信恢复尚未配置');
      } catch (error) { setStatus(status, error.message || '恢复服务不可用', true); }
    });
  }

  function installSecurityCenter() {
    const open = document.getElementById('accountIdentitiesRow');
    const summary = document.getElementById('accountSecuritySummary');
    const status = document.getElementById('accountSecurityStatus');
    const sessions = document.getElementById('accountSessionList');
    const change = document.getElementById('accountPasswordChange');
    const rotate = document.getElementById('accountRecoveryCodesRotate');
    const revokeOthers = document.getElementById('accountRevokeOthers');
    if (!open || !summary || !status || !sessions || !change || !rotate || !revokeOthers) return;
    let snapshot;
    async function refresh() {
      setStatus(status, '读取账号安全状态…');
      try {
        snapshot = await api('/api/auth/security');
        const changed = snapshot.password.changed_at ? ' · 修改于 ' + snapshot.password.changed_at : '';
        summary.textContent = (snapshot.password.enabled ? '密码已启用' : '尚未设置密码') + changed +
          '；可用离线恢复码 ' + snapshot.recovery.available_code_count + ' 个。';
        document.getElementById('accountCurrentPassword').style.display = snapshot.password.enabled ? '' : 'none';
        change.textContent = snapshot.password.enabled ? '修改密码' : '设置密码';
        renderSessions(snapshot.sessions || []);
        setStatus(status, '恢复码只显示一次；邮件与短信恢复尚未配置。');
      } catch (error) { setStatus(status, error.message || '读取失败', true); }
    }
    function renderSessions(values) {
      sessions.replaceChildren(...values.map((session) => {
        const row = document.createElement('div');
        row.className = 'account-session-row';
        const copy = document.createElement('span');
        copy.className = 'account-session-copy';
        const name = document.createElement('strong');
        name.textContent = (session.device_name || '未命名设备') + (session.current ? ' · 当前设备' : '');
        const detail = document.createElement('span');
        detail.textContent = (session.trusted_device ? '已信任 · ' : '') + (session.last_seen_at || '尚无最近活动时间');
        copy.append(name, detail);
        const revoke = document.createElement('button');
        revoke.type = 'button';
        revoke.textContent = session.current ? '退出' : '撤销';
        revoke.addEventListener('click', async () => {
          if (!confirm('确定撤销这个登录会话？')) return;
          try {
            await api('/api/auth/sessions/' + encodeURIComponent(session.id), { method: 'DELETE' });
            if (session.current) {
              localStorage.removeItem(TOKEN_KEY); localStorage.removeItem('elon_token'); location.reload();
            } else await refresh();
          } catch (error) { setStatus(status, error.message || '撤销失败', true); }
        });
        row.append(copy, revoke);
        return row;
      }));
      revokeOthers.disabled = !values.some((session) => !session.current);
    }
    open.addEventListener('click', refresh);
    change.addEventListener('click', async () => {
      const current = document.getElementById('accountCurrentPassword').value;
      const password = document.getElementById('accountNewPassword').value;
      const confirmPassword = document.getElementById('accountConfirmPassword').value;
      if (snapshot && snapshot.password.enabled && !current) return setStatus(status, '请输入当前密码', true);
      if (password.length < 8) return setStatus(status, '新密码至少 8 位', true);
      if (password !== confirmPassword) return setStatus(status, '两次输入的新密码不一致', true);
      try {
        await api('/api/auth/password', { method: 'PUT', body: JSON.stringify({
          current_password: current || null, new_password: password, request_id: requestId('password'), confirm: true
        }) });
        document.getElementById('accountCurrentPassword').value = '';
        document.getElementById('accountNewPassword').value = '';
        document.getElementById('accountConfirmPassword').value = '';
        setStatus(status, '密码已更新，其他设备会话已撤销。');
        await refresh();
      } catch (error) { setStatus(status, error.message || '密码更新失败', true); }
    });
    rotate.addEventListener('click', async () => {
      const current = document.getElementById('accountCurrentPassword').value;
      if (snapshot && snapshot.password.enabled && !current) return setStatus(status, '请先输入当前密码', true);
      if (!confirm('旧恢复码会立即失效，新恢复码只显示一次。确定继续？')) return;
      try {
        const result = await api('/api/auth/recovery-codes/rotate', { method: 'POST', body: JSON.stringify({
          current_password: current || null, request_id: requestId('recovery-codes'), confirm: true
        }) });
        const codes = (result.result && result.result.codes) || [];
        const output = document.getElementById('accountRecoveryCodes');
        output.textContent = codes.join('\n') || '恢复码未再次返回，请重新生成。';
        output.classList.add('active');
        setStatus(status, '请立即复制保存；关闭后页面不会再次显示。');
        await refresh();
      } catch (error) { setStatus(status, error.message || '恢复码生成失败', true); }
    });
    document.getElementById('accountRecoveryCodesCopy').addEventListener('click', async () => {
      const output = document.getElementById('accountRecoveryCodes');
      if (output.textContent) await navigator.clipboard.writeText(output.textContent);
      setStatus(status, '恢复码已复制到剪贴板。');
    });
    revokeOthers.addEventListener('click', async () => {
      if (!confirm('保留当前设备并退出其他全部设备？')) return;
      try {
        const result = await api('/api/auth/sessions/revoke-others', {
          method: 'POST', body: JSON.stringify({ confirm: true })
        });
        setStatus(status, '已撤销 ' + result.revoked_session_count + ' 个会话。');
        await refresh();
      } catch (error) { setStatus(status, error.message || '操作失败', true); }
    });
  }

  document.addEventListener('DOMContentLoaded', () => {
    installLoginRecovery();
    installSecurityCenter();
  });
})();
