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

  function friendlyError(body, status) {
    const messages = {
      google_oidc_not_configured: 'Google 登录尚未配置，暂时无法绑定',
      identity_owned_by_another_account: '这个 Google 账号已绑定到另一一龙账号，不能自动合并；请先在原账号解绑',
      existing_account_requires_bind: '该账号已存在，请先登录原一龙账号后再主动绑定',
      invalid_or_consumed_challenge: '本次 Google 登录已过期，请重新发起',
      auth_rate_limited: '操作过于频繁，请稍后再试',
      cannot_unlink_last_login: '这是账号最后一种登录方式，请先设置密码或绑定其他方式',
      google_jwks_unavailable: 'Google 身份验证暂时不可用，请稍后再试',
      identity_service_unavailable: '身份服务暂时不可用，请稍后再试'
    };
    return messages[body.code] || body.error || '请求失败 (' + status + ')';
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
      throw new Error(friendlyError(body, response.status));
    }
    return response.status === 204 ? null : response.json();
  }

  function maskedAccount(value) {
    const text = String(value || '').trim();
    if (!text) return '当前登录账号';
    if (/^\d{7,}$/.test(text)) return text.slice(0, 3) + '****' + text.slice(-2);
    return text.includes('@') ? maskedEmail(text) : text;
  }

  function maskedEmail(value) {
    const text = String(value || '').trim();
    const at = text.indexOf('@');
    if (at <= 0 || at === text.length - 1) return text || 'Google 身份';
    const local = text.slice(0, at);
    const visible = local.length === 1
      ? local[0] + '***'
      : local.length === 2
        ? local[0] + '***' + local[1]
        : local.slice(0, 2) + '***' + local.slice(-1);
    return visible + text.slice(at);
  }

  function currentUser() {
    return window.ElonWebApp && window.ElonWebApp.getCurrentUser
      ? window.ElonWebApp.getCurrentUser()
      : null;
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

  async function renderGoogleButton(host, status, mode, onComplete, knownProvider) {
    status.classList.remove('error', 'unavailable');
    status.textContent = '正在检查 Google 登录…';
    try {
      let provider = knownProvider;
      if (!provider) {
        const config = await api('/api/auth/federation/providers');
        provider = (config.providers || []).find((item) => item.id === 'google');
      }
      if (!provider || !provider.configured || !provider.client_id) {
        status.classList.add('unavailable');
        status.textContent = 'Google 登录尚未配置，暂时无法绑定';
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
          status.textContent = mode === 'login' ? '正在登录一龙账号…' : '正在绑定到当前一龙账号…';
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
      status.textContent = mode === 'login'
        ? '使用自己的 Google 账号登录'
        : '继续即表示把所选 Google 账号绑定到当前一龙账号';
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
    const subtitle = document.getElementById('accountIdentitiesSubtitle');
    const mask = document.getElementById('accountIdentityMask');
    const close = document.getElementById('accountIdentityClose');
    const account = document.getElementById('accountIdentityCurrentAccount');
    const state = document.getElementById('accountGoogleBindingState');
    const list = document.getElementById('accountIdentityList');
    const host = document.getElementById('federatedBindGoogle');
    const status = document.getElementById('federatedBindStatus');
    if (!open || !subtitle || !mask || !close || !account || !state || !list || !host || !status) return;

    const setState = (label, className) => {
      state.textContent = label;
      state.className = 'identity-state' + (className ? ' ' + className : '');
    };
    const setAccount = () => {
      const user = currentUser();
      account.textContent = maskedAccount(user && (user.account || user.nickname));
    };
    const hide = () => {
      mask.classList.remove('active');
      mask.setAttribute('aria-hidden', 'true');
    };
    close.addEventListener('click', hide);
    mask.addEventListener('click', (event) => { if (event.target === mask) hide(); });

    async function loadIdentityState(showGoogleButton) {
      setAccount();
      list.innerHTML = '<div class="federated-status">读取中…</div>';
      host.replaceChildren();
      status.className = 'federated-status';
      status.textContent = '';
      setState('读取中', '');
      try {
        const [identityData, providerData] = await Promise.all([
          api('/api/auth/identities'),
          api('/api/auth/federation/providers')
        ]);
        const identities = identityData.identities || [];
        const googleIdentity = identities.find((item) => item.provider === 'google');
        const provider = (providerData.providers || []).find((item) => item.id === 'google');
        list.replaceChildren(...identities.map(identityRow));
        if (googleIdentity) {
          setState('已绑定', 'bound');
          subtitle.textContent = 'Google 已绑定 · ' + maskedEmail(googleIdentity.email);
          status.textContent = 'Google 已绑定到当前一龙账号，可作为登录方式使用';
        } else if (!provider || !provider.configured || !provider.client_id) {
          setState('暂未配置', 'unavailable');
          subtitle.textContent = 'Google 暂未配置';
          status.classList.add('unavailable');
          status.textContent = 'Google 登录尚未配置，暂时无法绑定';
        } else {
          setState('未绑定', '');
          subtitle.textContent = 'Google 未绑定 · 点击设置';
          if (showGoogleButton) {
            await renderGoogleButton(host, status, 'bind', () => loadIdentityState(true), provider);
          }
        }
      } catch (error) {
        setState('暂不可用', 'unavailable');
        subtitle.textContent = 'Google 绑定状态暂不可用';
        list.replaceChildren();
        status.classList.add('error');
        status.textContent = error.message || '读取失败';
      }
    }

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
          await loadIdentityState(true);
        } catch (error) {
          status.classList.add('error');
          status.textContent = error.message || '解绑失败';
        }
      });
      row.append(avatar, copy, unlink);
      return row;
    }

    open.addEventListener('click', async () => {
      mask.classList.add('active');
      mask.setAttribute('aria-hidden', 'false');
      await loadIdentityState(true);
    });
    if (token()) loadIdentityState(false);
  }

  document.addEventListener('DOMContentLoaded', () => {
    installLogin();
    installIdentitySheet();
  });
})();
