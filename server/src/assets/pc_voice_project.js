(function () {
  const CHANNELS = [
    { id: 'studio', glyph: '声', title: '声音控制台', sub: '试听、声线与中继' },
    { id: 'training', glyph: '训', title: '训练方案', sub: '设置自己的 ai 声音' },
    { id: 'sdk', glyph: 'SDK', title: '对外 SDK', sub: 'API、JS SDK、接入示例' }
  ];
  const PLAN_KEY = 'elon_voice_training_plan';

  function esc(ctx, value) {
    return ctx.escapeHtml(value == null ? '' : value);
  }

  function clean(ctx, value) {
    return ctx.clean ? ctx.clean(value) : String(value || '').trim();
  }

  function client(ctx) {
    return window.ElonVoiceTts.createClient({ tokenProvider: () => ctx.state.token });
  }

  function localUrl(ctx, path) {
    return new URL(path.replace(/^\//, ''), ctx.state.nodeAdminUrl).toString();
  }

  async function localJson(ctx, path, options) {
    const resp = await fetch(localUrl(ctx, path), Object.assign({
      mode: 'cors',
      headers: { 'Content-Type': 'application/json' }
    }, options || {}));
    const data = await resp.json().catch(() => ({}));
    if (!resp.ok) throw new Error(data.error || data.message || resp.statusText);
    return data;
  }

  function channelButton(ctx, channel) {
    const active = (ctx.state.activeVoiceChannel || 'studio') === channel.id;
    return `<button class="channel-item ${active ? 'active' : ''}" type="button" data-voice-channel="${esc(ctx, channel.id)}">
      <span class="glyph">${esc(ctx, channel.glyph)}</span>
      <span class="main"><strong>${esc(ctx, channel.title)}</strong><span>${esc(ctx, channel.sub)}</span></span>
    </button>`;
  }

  function renderChannels(ctx) {
    ctx.els.channelList.innerHTML = [
      '<div class="channel-section">ai声音</div>',
      CHANNELS.map((channel) => channelButton(ctx, channel)).join('')
    ].join('');
    ctx.els.channelList.querySelectorAll('[data-voice-channel]').forEach((button) => {
      button.addEventListener('click', () => ctx.selectVoiceChannel(button.dataset.voiceChannel));
    });
  }

  async function renderMain(ctx) {
    const channel = ctx.state.activeVoiceChannel || 'studio';
    ctx.setNodeMode(false);
    ctx.setComposer(false, 'ai声音项目中配置', false);
    ctx.renderMembers('ai声音项目', [
      { name: '声音控制台', sub: '声线、情绪、试听' },
      { name: '训练方案', sub: '授权素材与模型 Worker' },
      { name: 'SDK', sub: '给外部应用调用 TTS' }
    ]);
    if (channel === 'training') return renderTraining(ctx);
    if (channel === 'sdk') return renderSdk(ctx);
    return renderStudio(ctx);
  }

  function page(title, body) {
    return `<div class="voice-page">
      <div class="voice-panel">
        <h3>${title}</h3>
        ${body}
      </div>
    </div>`;
  }

  async function renderStudio(ctx) {
    ctx.setHeader('声', 'ai声音', '女声情绪 TTS、试听与本机中继');
    ctx.els.messageList.innerHTML = page('声音控制台', '<p>正在读取云端目录和本机 TTS Worker 状态…</p>');
    const [catalogResult, statusResult, relayResult] = await Promise.allSettled([
      client(ctx).catalog(),
      localJson(ctx, '/api/tts-status'),
      localJson(ctx, '/api/tts-relay-config')
    ]);
    const catalog = catalogResult.status === 'fulfilled' ? catalogResult.value : null;
    const status = statusResult.status === 'fulfilled' ? statusResult.value : null;
    const relay = relayResult.status === 'fulfilled' ? relayResult.value : {};
    ctx.els.messageList.innerHTML = studioHtml(ctx, catalog, status, relay);
    bindStudio(ctx, catalog);
  }

  function studioHtml(ctx, catalog, status, relay) {
    const voices = (catalog && catalog.voices) || [];
    const emotions = (catalog && catalog.emotions) || [];
    const intensities = (catalog && catalog.intensities) || [];
    const workerOk = !!(catalog && catalog.workerConfigured);
    const localRunning = !!(status && status.running);
    const localHealth = (status && status.health) || {};
    const provider = clean(ctx, localHealth.defaultProvider || catalog && catalog.defaultProvider || 'auto');
    return `<div class="voice-page">
      <div class="voice-hero">
        <section class="voice-panel">
          <h3>ai声音</h3>
          <p>这里集中管理情绪女声 TTS：云端目录、PC 本机模型 Worker、中继地址、试听和外部 SDK。分享算力页只保留算力分享与电脑维护。</p>
          <div class="voice-action-row">
            <span class="voice-status ${workerOk ? 'ok' : 'bad'}">${workerOk ? '云端可合成' : '云端未配置'}</span>
            <span class="voice-status ${localRunning ? 'ok' : 'warn'}">${localRunning ? '本机 Worker 运行中' : '本机 Worker 未运行'}</span>
            <span class="voice-status">引擎 ${esc(ctx, provider || 'auto')}</span>
          </div>
        </section>
        <section class="voice-panel">
          <h3>本机 TTS 中继</h3>
          <p>把本机 GPU / 模型 Worker 贡献给平台。默认模型 Worker 地址是 <code>http://127.0.0.1:5011</code>。</p>
          <div class="voice-field">
            <label>本机 TTS Worker 地址</label>
            <input id="voiceRelayUrl" placeholder="http://127.0.0.1:5011" value="${esc(ctx, relay.ttsWorkerUrl || '')}" />
          </div>
          <div class="voice-action-row">
            <button class="voice-button" id="voiceSaveRelay" type="button">保存中继</button>
            <button class="voice-button secondary" id="voiceRefreshStudio" type="button">刷新</button>
          </div>
          <div class="voice-result" id="voiceRelayResult"></div>
        </section>
      </div>
      <div class="voice-hero">
        <section class="voice-panel">
          <h3>试听</h3>
          <div class="voice-field">
            <label>朗读文本</label>
            <textarea id="voicePreviewText" rows="4">你好，我是一龙的 ai 声音。今天想用什么情绪陪你说话？</textarea>
          </div>
          <div class="voice-grid">
            ${selectField(ctx, 'voiceVoice', '声线', voices, 'female_warm')}
            ${selectField(ctx, 'voiceEmotion', '情绪', emotions, 'normal')}
            ${selectField(ctx, 'voiceIntensity', '强度', intensities, 'normal')}
          </div>
          <div class="voice-action-row">
            <button class="voice-button" id="voicePlayPreview" type="button">试听声音</button>
          </div>
          <div class="voice-result" id="voicePreviewResult"></div>
        </section>
        <section class="voice-panel">
          <h3>目录</h3>
          <div class="voice-list">
            ${voices.slice(0, 5).map((item) => `<div class="voice-item"><strong>${esc(ctx, item.label)}</strong><span>${esc(ctx, item.description)}</span></div>`).join('') || '<p>暂无声线目录</p>'}
          </div>
        </section>
      </div>
    </div>`;
  }

  function selectField(ctx, id, label, items, fallback) {
    return `<div class="voice-field"><label>${esc(ctx, label)}</label><select id="${id}">
      ${(items || []).map((item) => `<option value="${esc(ctx, item.id)}" ${item.id === fallback ? 'selected' : ''}>${esc(ctx, item.label || item.id)}</option>`).join('')}
    </select></div>`;
  }

  function bindStudio(ctx, catalog) {
    const save = document.getElementById('voiceSaveRelay');
    const refresh = document.getElementById('voiceRefreshStudio');
    const play = document.getElementById('voicePlayPreview');
    if (save) save.addEventListener('click', () => saveRelay(ctx));
    if (refresh) refresh.addEventListener('click', () => renderStudio(ctx));
    if (play) play.addEventListener('click', () => playPreview(ctx, catalog));
  }

  async function saveRelay(ctx) {
    const out = document.getElementById('voiceRelayResult');
    const url = clean(ctx, document.getElementById('voiceRelayUrl') && document.getElementById('voiceRelayUrl').value);
    out.textContent = '保存中…';
    try {
      const data = await localJson(ctx, '/api/tts-relay-config', {
        method: 'POST',
        body: JSON.stringify({ tts_worker_url: url || null })
      });
      out.textContent = data.ttsWorkerUrl ? '已启用中继：' + data.ttsWorkerUrl : '已禁用中继';
    } catch (error) {
      out.textContent = '保存失败：' + (error.message || error);
    }
  }

  async function playPreview(ctx) {
    const out = document.getElementById('voicePreviewResult');
    out.textContent = '合成中…';
    try {
      const result = await client(ctx).play({
        text: clean(ctx, document.getElementById('voicePreviewText').value),
        voiceId: document.getElementById('voiceVoice').value,
        emotionId: document.getElementById('voiceEmotion').value,
        intensity: document.getElementById('voiceIntensity').value
      });
      out.textContent = `播放中：${result.provider || 'tts'} / ${result.voice || 'voice'} / ${result.emotion || 'emotion'}`;
    } catch (error) {
      out.textContent = '试听失败：' + (error.message || error);
    }
  }

  function renderTraining(ctx) {
    ctx.setHeader('训', '训练自己的 ai 声音', '授权素材、声线档案和模型 Worker');
    const plan = loadPlan();
    ctx.els.messageList.innerHTML = `<div class="voice-page">
      <div class="voice-hero">
        <section class="voice-panel">
          <h3>训练方案</h3>
          <div class="voice-field"><label>声音名称</label><input id="voicePlanName" value="${esc(ctx, plan.name || '我的 ai 声音')}" /></div>
          <div class="voice-field"><label>模型引擎</label><select id="voicePlanEngine">
            ${option(ctx, 'index_tts2', 'IndexTTS2：强情绪参考', plan.engine)}
            ${option(ctx, 'cosyvoice3', 'CosyVoice3：自然克隆', plan.engine)}
            ${option(ctx, 'gpt_sovits', 'GPT-SoVITS：自定义训练', plan.engine)}
          </select></div>
          <div class="voice-field"><label>授权声音素材目录</label><input id="voicePlanSamples" value="${esc(ctx, plan.samples || 'D:\\authorized-tts-samples')}" /></div>
          <div class="voice-field"><label>资产输出目录</label><input id="voicePlanAssetRoot" value="${esc(ctx, plan.assetRoot || 'D:\\tts-assets')}" /></div>
          <div class="voice-action-row"><button class="voice-button" id="voiceSavePlan" type="button">保存训练方案</button></div>
          <div class="voice-result" id="voicePlanResult"></div>
        </section>
        <section class="voice-panel">
          <h3>训练步骤</h3>
          <div class="voice-list">
            <div class="voice-item"><strong>1. 授权采样</strong><span>只上传本人或已授权的 10-30 分钟干净人声，不使用明星、主播或声优声音。</span></div>
            <div class="voice-item"><strong>2. 导入资产</strong><span>用导入脚本转成 24kHz mono wav，并生成 voices / emotions 目录。</span></div>
            <div class="voice-item"><strong>3. 启动 Worker</strong><span>本机运行模型 Worker，再回到声音控制台启用中继。</span></div>
          </div>
        </section>
      </div>
      <section class="voice-panel">
        <h3>推荐命令</h3>
        <pre class="voice-code">powershell -ExecutionPolicy Bypass -File scripts\\import-tts-asset-pack.ps1 -AssetRoot "${esc(ctx, plan.assetRoot || 'D:\\tts-assets')}" -SourceDir "${esc(ctx, plan.samples || 'D:\\authorized-tts-samples')}" -FailOnMissing
powershell -ExecutionPolicy Bypass -File scripts\\start-local-model-tts-worker.ps1 -Provider ${esc(ctx, plan.engine || 'index_tts2')} -AssetRoot "${esc(ctx, plan.assetRoot || 'D:\\tts-assets')}"</pre>
      </section>
    </div>`;
    const save = document.getElementById('voiceSavePlan');
    if (save) save.addEventListener('click', () => savePlan(ctx));
  }

  function option(ctx, value, label, selected) {
    return `<option value="${esc(ctx, value)}" ${value === selected ? 'selected' : ''}>${esc(ctx, label)}</option>`;
  }

  function loadPlan() {
    try { return JSON.parse(localStorage.getItem(PLAN_KEY) || '{}') || {}; } catch (_) { return {}; }
  }

  function savePlan(ctx) {
    const plan = {
      name: clean(ctx, document.getElementById('voicePlanName').value),
      engine: document.getElementById('voicePlanEngine').value,
      samples: clean(ctx, document.getElementById('voicePlanSamples').value),
      assetRoot: clean(ctx, document.getElementById('voicePlanAssetRoot').value)
    };
    localStorage.setItem(PLAN_KEY, JSON.stringify(plan));
    document.getElementById('voicePlanResult').textContent = '训练方案已保存在本机浏览器。';
  }

  function renderSdk(ctx) {
    ctx.setHeader('SDK', 'ai声音 SDK', '把 TTS 能力开放给外部应用');
    const code = `<script src="${location.origin}/assets/voice_tts_sdk.js"></script>
<script>
const tts = ElonVoiceTts.createClient({
  baseUrl: '${location.origin}',
  token: '<用户登录 token>'
});
await tts.play({
  text: '你好，我是你的 ai 声音。',
  voiceId: 'female_warm',
  emotionId: 'gentle_comfort',
  intensity: 'normal'
});
</script>`;
    ctx.els.messageList.innerHTML = `<div class="voice-page">
      <div class="voice-hero">
        <section class="voice-panel">
          <h3>外部服务能力</h3>
          <p><code>GET /api/voice/tts/catalog</code> 返回声线、情绪、强度目录。</p>
          <p><code>POST /api/voice/tts</code> 返回音频 Blob，支持云端 Worker 或用户 PC 节点模型 Worker。</p>
          <p><code>/assets/voice_tts_sdk.js</code> 封装鉴权、目录读取、音频合成和浏览器播放。</p>
        </section>
        <section class="voice-panel">
          <h3>接入示例</h3>
          <pre class="voice-code" id="voiceSdkCode">${esc(ctx, code)}</pre>
          <div class="voice-action-row"><button class="voice-button" id="voiceCopySdk" type="button">复制示例</button></div>
          <div class="voice-result" id="voiceSdkResult"></div>
        </section>
      </div>
    </div>`;
    const copy = document.getElementById('voiceCopySdk');
    if (copy) copy.addEventListener('click', async () => {
      await navigator.clipboard.writeText(code);
      document.getElementById('voiceSdkResult').textContent = '已复制 SDK 示例。';
    });
  }

  window.ElonVoiceProject = {
    renderChannels,
    renderMain
  };
})();
