(function () {
  'use strict';
  const TOKEN_KEY = 'lodex_token';
  const CLIENT_KEY = 'elon_federated_auth_client_instance_id';
  const token = () => localStorage.getItem(TOKEN_KEY) || localStorage.getItem('elon_token') || '';
  let googleScript;

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
    if (!response.ok) {
      const body = await response.json().catch(() => ({}));
      throw new Error(body.error || '请求失败 (' + response.status + ')');
    }
    return response.status === 204 ? null : response.json();
  }

  function loadGoogle() {
    if (window.google && window.google.accounts && window.google.accounts.id) return Promise.resolve();
    if (googleScript) return googleScript;
    googleScript = new Promise((resolve, reject) => {
      const script = document.createElement('script');
      script.src = 'https://accounts.google.com/gsi/client';
      script.async = true;
      script.defer = true;
      script.onload = resolve;
      script.onerror = () => reject(new Error('无法加载 Google 官方登录组件'));
      document.head.appendChild(script);
    });
    return googleScript;
  }

  async function renderGoogleButton(host, status, mode, onComplete) {
    status.classList.remove('error');
    status.textContent = '正在检查 Google 登录…';
    try {
      const config = await api('/api/auth/federation/providers');
      const provider = (config.providers || []).find((item) => item.id === 'google');
      if (!provider || !provider.configured || !provider.client_id) {
        status.textContent = 'Google 登录等待管理员配置客户端 ID';
        return;
      }
      const challenge = await api('/api/auth/federation/google/challenges', {
        method: 'POST',
        body: JSON.stringify({
          mode, platform: 'web', request_id: 'web:challenge:' + randomId(),
          client_instance_id: clientInstanceId()
        })
      });
      const completionRequestId = 'web:complete:' + randomId();
      await loadGoogle();
      host.replaceChildren();
      window.google.accounts.id.initialize({
        client_id: provider.client_id,
        nonce: challenge.nonce,
        callback: async (credential) => {
          if (!credential.credential) return;
          status.textContent = mode === 'login' ? '正在登录一龙账号…' : '正在绑定账号…';
          try {
            const result = await api('/api/auth/federation/google/complete', {
              method: 'POST',
              body: JSON.stringify({
                challenge_id: challenge.id,
                id_token: credential.credential,
                remember_device: true,
                device_name: 'Mobile Web',
                request_id: completionRequestId,
                client_instance_id: clientInstanceId()
              })
            });
            await onComplete(result);
          } catch (error) {
            status.classList.add('error');
            status.textContent = error.message || 'Google 登录失败';
          }
        }
      });
      window.google.accounts.id.renderButton(host, {
        type: 'standard', theme: 'outline', size: 'large', width: 300,
        text: mode === 'login' ? 'signin_with' : 'continue_with', shape: 'rectangular'
      });
      status.textContent = mode === 'login' ? '使用自己的 Google 账号登录' : '绑定后可用同一一龙账号登录';
    } catch (error) {
      status.classList.add('error');
      status.textContent = error.message || 'Google 登录不可用';
    }
  }

  function installLogin() {
    const host = document.getElementById('federatedLoginGoogle');
    const status = document.getElementById('federatedLoginStatus');
    if (!host || !status || token()) return;
    const block = host.closest('.federated-login');
    document.querySelectorAll('[data-auth-mode]').forEach((button) => {
      button.addEventListener('click', () => {
        if (block) block.style.display = button.dataset.authMode === 'login' ? '' : 'none';
      });
    });
    renderGoogleButton(host, status, 'login', async (result) => {
      if (!result.session || !result.session.token) throw new Error('服务端没有创建登录会话');
      localStorage.setItem(TOKEN_KEY, result.session.token);
      localStorage.removeItem('elon_token');
      window.location.reload();
    });
  }

  function installIdentitySheet() {
    const open = document.getElementById('accountIdentitiesRow');
    const mask = document.getElementById('accountIdentityMask');
    const close = document.getElementById('accountIdentityClose');
    const list = document.getElementById('accountIdentityList');
    const host = document.getElementById('federatedBindGoogle');
    const status = document.getElementById('federatedBindStatus');
    if (!open || !mask || !close || !list || !host || !status) return;
    const hide = () => mask.classList.remove('active');
    close.addEventListener('click', hide);
    mask.addEventListener('click', (event) => { if (event.target === mask) hide(); });
    open.addEventListener('click', async () => {
      mask.classList.add('active');
      list.innerHTML = '<div class="federated-status">读取中…</div>';
      host.replaceChildren();
      try {
        const data = await api('/api/auth/identities');
        const identities = data.identities || [];
        list.replaceChildren(...identities.map(identityRow));
        if (!identities.some((identity) => identity.provider === 'google')) {
          renderGoogleButton(host, status, 'bind', async () => open.click());
        } else {
          status.textContent = 'Google 账号已绑定';
        }
      } catch (error) {
        list.innerHTML = '<div class="federated-status error"></div>';
        list.firstElementChild.textContent = error.message || '读取失败';
      }
    });
    function identityRow(identity) {
      const row = document.createElement('div');
      row.className = 'identity-row';
      const avatar = identity.avatar_url
        ? Object.assign(document.createElement('img'), { src: identity.avatar_url, alt: '' })
        : Object.assign(document.createElement('span'), { className: 'identity-provider-icon', textContent: 'G' });
      if (avatar.tagName === 'IMG') avatar.referrerPolicy = 'no-referrer';
      const copy = document.createElement('span');
      copy.className = 'identity-row-copy';
      const name = document.createElement('strong');
      name.textContent = identity.display_name || 'Google';
      const email = document.createElement('span');
      email.textContent = identity.email || '已绑定 Google 身份';
      copy.append(name, email);
      const unlink = document.createElement('button');
      unlink.type = 'button';
      unlink.textContent = '解绑';
      unlink.addEventListener('click', async () => {
        if (!window.confirm('确定解绑 ' + email.textContent + '？')) return;
        try {
          await api('/api/auth/identities/' + encodeURIComponent(identity.id), { method: 'DELETE' });
          open.click();
        } catch (error) {
          status.classList.add('error');
          status.textContent = error.message || '解绑失败';
        }
      });
      row.append(avatar, copy, unlink);
      return row;
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    installLogin();
    installIdentitySheet();
  });
})();
