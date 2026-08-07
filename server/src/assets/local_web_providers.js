(function () {
  'use strict';

  const providers = Object.freeze([
    Object.freeze({
      id: 'chatgpt_web',
      label: 'ChatGPT 账号与聊天',
      officialUrl: 'https://chatgpt.com/',
      semanticProtocol: 'yilong.ai.ui.v1',
      capabilities: Object.freeze({
        officialWeb: true,
        localBrowserSession: true,
        nativeProjectionInPwa: false,
        nativeProjectionInApk: true
      }),
      detail: '打开 ChatGPT 官方页面后，由本人完成登录。登录与 Cookie 由当前浏览器保管，不会保存到一龙云端账号。'
    })
  ]);

  window.ElonLocalWebProviders = Object.freeze({
    list: function () { return providers.slice(); },
    find: function (providerId) {
      return providers.find(function (provider) { return provider.id === providerId; }) || null;
    }
  });
})();
