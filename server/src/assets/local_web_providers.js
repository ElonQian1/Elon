(function () {
  'use strict';

  const providers = Object.freeze([
    Object.freeze({
      id: 'chatgpt_web',
      label: 'ChatGPT 网页',
      officialUrl: 'https://chatgpt.com/',
      semanticProtocol: 'yilong.ai.ui.v1',
      capabilities: Object.freeze({
        officialWeb: true,
        localBrowserSession: true,
        nativeProjectionInPwa: false,
        nativeProjectionInApk: true
      }),
      detail: '登录与 Cookie 由当前浏览器保管。纯 PWA 受同源隔离，不能读取或重新呈现 ChatGPT 页面；APK 可使用本地网页增强模式。'
    })
  ]);

  window.ElonLocalWebProviders = Object.freeze({
    list: function () { return providers.slice(); },
    find: function (providerId) {
      return providers.find(function (provider) { return provider.id === providerId; }) || null;
    }
  });
})();
