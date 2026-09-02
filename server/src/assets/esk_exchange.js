(() => {
  'use strict';

  const bridge = globalThis.yilongEskExchangeBridge;
  const card = document.getElementById('profileEskAssetCard');
  const statusAnchor = document.getElementById('profileEskStatus');
  if (!bridge || !card || !statusAnchor || typeof bridge.api !== 'function') return;

  const panel = document.createElement('div');
  panel.className = 'esk-exchange-panel';
  panel.setAttribute('aria-label', 'ESK 与 USDT Paper 模拟兑换');
  panel.innerHTML = `
    <div class="esk-exchange-heading">
      <strong>ESK / USDT 兑换</strong>
      <span class="esk-exchange-paper-badge">Paper 模拟 · 未上链</span>
    </div>
    <div class="esk-exchange-balances">
      <div><span>Paper USDT 可用</span><strong data-role="usdt-balance">—</strong><small>不是链上 USDT</small></div>
      <div><span>ESK 可用</span><strong data-role="esk-balance">—</strong><small>扣除其他申请占用</small></div>
    </div>
    <p class="esk-exchange-rate" data-role="rate">正在读取兑换配置…</p>
    <div class="esk-exchange-form">
      <select data-role="direction" aria-label="兑换方向">
        <option value="usdt_to_esk">USDT 买入 ESK</option>
        <option value="esk_to_usdt">ESK 换为 USDT</option>
      </select>
      <input data-role="amount" inputmode="decimal" autocomplete="off" placeholder="输入 USDT 数量" aria-label="兑换数量" />
      <button class="esk-exchange-button secondary" data-role="quote-button" type="button" disabled>获取精确报价</button>
    </div>
    <div class="esk-exchange-quote" data-role="quote" hidden>
      <div class="esk-exchange-quote-row"><span>投入</span><strong data-role="quote-input">—</strong></div>
      <div class="esk-exchange-quote-row"><span>兑换前金额</span><strong data-role="quote-gross">—</strong></div>
      <div class="esk-exchange-quote-row"><span>平台手续费</span><strong data-role="quote-fee">—</strong></div>
      <div class="esk-exchange-quote-row"><span>预计到账</span><strong data-role="quote-net">—</strong></div>
      <div class="esk-exchange-quote-row"><span>报价有效期</span><strong data-role="quote-expiry">—</strong></div>
    </div>
    <div class="esk-exchange-actions">
      <button class="esk-exchange-button" data-role="confirm-button" type="button" disabled>确认 Paper 模拟兑换</button>
      <button class="esk-exchange-button secondary" data-role="refresh-button" type="button">刷新账户</button>
    </div>
    <p class="esk-exchange-message" data-role="message" role="status"></p>
    <p class="esk-exchange-safety">此入口只变更 Paper 模拟账本：未上链、不移动真实资金；不会接收、发送或托管真实 USDT，也不代表真实成交或兑付。</p>
    <div class="esk-exchange-history" data-role="history" hidden></div>`;
  statusAnchor.insertAdjacentElement('afterend', panel);

  const find = (role) => panel.querySelector(`[data-role="${role}"]`);
  const controls = {
    usdt: find('usdt-balance'), esk: find('esk-balance'), rate: find('rate'),
    direction: find('direction'), amount: find('amount'), quoteButton: find('quote-button'),
    quote: find('quote'), quoteInput: find('quote-input'), quoteGross: find('quote-gross'),
    quoteFee: find('quote-fee'), quoteNet: find('quote-net'), quoteExpiry: find('quote-expiry'),
    confirmButton: find('confirm-button'), refreshButton: find('refresh-button'),
    message: find('message'), history: find('history')
  };
  let account = null;
  let quote = null;

  function isSafeEnvelope(value) {
    return value && value.simulated === true && value.funds_moved === false &&
      value.on_chain_settlement === false && value.trading_mode === 'paper';
  }

  function isExactAmount(value) {
    return typeof value === 'string' && /^\d+\.\d{6}$/.test(value);
  }

  function setMessage(message, error = false) {
    controls.message.textContent = message || '';
    controls.message.classList.toggle('error', error);
  }

  function clearQuote() {
    quote = null;
    controls.quote.hidden = true;
    controls.confirmButton.disabled = true;
  }

  function validateAccount(value) {
    if (!value || value.schema !== 'yilong.esk.paper_exchange_account.v1' ||
        !isSafeEnvelope(value) || !value.balances || !value.balances.esk ||
        !value.balances.usdt || !isExactAmount(value.balances.esk.available) ||
        !isExactAmount(value.balances.usdt.available)) {
      throw new Error('兑换账户安全标识或金额格式不匹配');
    }
    return value;
  }

  function validateQuote(value) {
    const amountFields = ['input_amount', 'gross_output_amount', 'fee_amount', 'net_output_amount', 'usdt_per_esk'];
    if (!value || value.schema !== 'yilong.esk.paper_exchange_quote.v1' ||
        !isSafeEnvelope(value) || !value.quote_id || !amountFields.every((key) => isExactAmount(value[key])) ||
        !['ESK', 'USDT'].includes(value.input_asset) || !['ESK', 'USDT'].includes(value.output_asset) ||
        !['ESK', 'USDT'].includes(value.fee_asset)) {
      throw new Error('报价安全标识或金额格式不匹配');
    }
    return value;
  }

  async function jsonRequest(path, options = {}) {
    const response = await bridge.api(path, Object.assign({ cache: 'no-store' }, options));
    const data = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(data.error || `请求失败：HTTP ${response.status}`);
    return data;
  }

  function renderAccount(value) {
    controls.usdt.textContent = `${value.balances.usdt.available} USDT`;
    controls.esk.textContent = `${value.balances.esk.available} ESK`;
    controls.quoteButton.disabled = !value.enabled;
    if (value.pricing) {
      controls.rate.textContent = `参考价 1 ESK = ${value.pricing.usdt_per_esk} USDT · 手续费 ${value.pricing.fee_percent} · 报价 60 秒有效`;
    } else {
      controls.rate.textContent = value.status_message || 'Paper 兑换尚未配置';
    }
  }

  function renderQuote(value) {
    controls.quoteInput.textContent = `${value.input_amount} ${value.input_asset}`;
    controls.quoteGross.textContent = `${value.gross_output_amount} ${value.output_asset}`;
    controls.quoteFee.textContent = `${value.fee_amount} ${value.fee_asset}`;
    controls.quoteNet.textContent = `${value.net_output_amount} ${value.output_asset}`;
    const expiry = new Date(value.expires_at);
    controls.quoteExpiry.textContent = Number.isNaN(expiry.getTime()) ? value.expires_at : expiry.toLocaleTimeString();
    controls.quote.hidden = false;
    controls.confirmButton.disabled = false;
  }

  function renderHistory(records) {
    controls.history.replaceChildren();
    if (!records.length) {
      controls.history.hidden = true;
      return;
    }
    const title = document.createElement('strong');
    title.textContent = '最近 Paper 兑换';
    controls.history.appendChild(title);
    records.slice(0, 5).forEach((record) => {
      if (!record || !isSafeEnvelope(record) || !record.quote) return;
      const row = document.createElement('div');
      row.className = 'esk-exchange-history-item';
      const summary = document.createElement('span');
      summary.textContent = `${record.quote.input_amount} ${record.quote.input_asset} → ${record.quote.net_output_amount} ${record.quote.output_asset}`;
      const time = document.createElement('span');
      time.textContent = new Date(record.executed_at).toLocaleString();
      row.append(summary, time);
      controls.history.appendChild(row);
    });
    controls.history.hidden = controls.history.childElementCount <= 1;
  }

  async function load() {
    clearQuote();
    if (!bridge.isLoggedIn()) {
      controls.usdt.textContent = '—';
      controls.esk.textContent = '—';
      controls.quoteButton.disabled = true;
      setMessage('登录后可查看 Paper 兑换账户');
      return;
    }
    controls.refreshButton.disabled = true;
    setMessage('正在同步 Paper 兑换账户…');
    try {
      const [accountData, historyData] = await Promise.all([
        jsonRequest('/api/me/assets/esk/exchange-account'),
        jsonRequest('/api/me/assets/esk/exchanges?limit=5')
      ]);
      account = validateAccount(accountData);
      renderAccount(account);
      if (!historyData || historyData.schema !== 'yilong.esk.paper_exchange_execution_list.v1' || !isSafeEnvelope(historyData)) {
        throw new Error('兑换流水安全标识不匹配');
      }
      renderHistory(Array.isArray(historyData.executions) ? historyData.executions : []);
      setMessage(account.status_message || 'Paper 兑换账户已同步');
    } catch (error) {
      account = null;
      controls.quoteButton.disabled = true;
      setMessage((error && error.message) || 'Paper 兑换账户暂不可用', true);
    } finally {
      controls.refreshButton.disabled = false;
    }
  }

  async function requestQuote() {
    clearQuote();
    const amount = String(controls.amount.value || '').trim();
    if (!/^\d+(\.\d{1,6})?$/.test(amount) || /^0+(\.0+)?$/.test(amount)) {
      setMessage('请输入大于 0、最多六位小数的数量', true);
      return;
    }
    controls.quoteButton.disabled = true;
    setMessage('正在生成 60 秒有效报价…');
    try {
      quote = validateQuote(await jsonRequest('/api/me/assets/esk/exchange-quotes', {
        method: 'POST',
        body: JSON.stringify({ direction: controls.direction.value, input_amount: amount })
      }));
      renderQuote(quote);
      setMessage('请核对兑换前金额、手续费与预计到账，再进行第二次确认。');
    } catch (error) {
      setMessage((error && error.message) || '报价失败，请稍后重试', true);
    } finally {
      controls.quoteButton.disabled = !(account && account.enabled);
    }
  }

  async function executeQuote() {
    if (!quote) return;
    controls.confirmButton.disabled = true;
    controls.quoteButton.disabled = true;
    setMessage('正在记入 Paper 模拟账本…');
    try {
      const suffix = globalThis.crypto && typeof globalThis.crypto.randomUUID === 'function'
        ? globalThis.crypto.randomUUID() : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
      const execution = await jsonRequest('/api/me/assets/esk/exchanges', {
        method: 'POST',
        body: JSON.stringify({
          quote_id: quote.quote_id,
          idempotency_key: `esk-pwa-exchange-${suffix}`,
          confirmation: 'CONFIRM PAPER ESK USDT EXCHANGE'
        })
      });
      if (!execution || execution.schema !== 'yilong.esk.paper_exchange_execution.v1' || !isSafeEnvelope(execution)) {
        throw new Error('兑换结果安全标识不匹配');
      }
      setMessage('Paper 模拟兑换已记账；没有移动真实资金。');
      controls.amount.value = '';
      clearQuote();
      await Promise.all([load(), bridge.reloadAsset()]);
    } catch (error) {
      setMessage((error && error.message) || 'Paper 兑换失败，请刷新后重试', true);
      controls.confirmButton.disabled = false;
      controls.quoteButton.disabled = !(account && account.enabled);
    }
  }

  controls.direction.addEventListener('change', () => {
    controls.amount.placeholder = controls.direction.value === 'usdt_to_esk' ? '输入 USDT 数量' : '输入 ESK 数量';
    clearQuote();
  });
  controls.amount.addEventListener('input', clearQuote);
  controls.quoteButton.addEventListener('click', requestQuote);
  controls.confirmButton.addEventListener('click', executeQuote);
  controls.refreshButton.addEventListener('click', () => Promise.all([load(), bridge.reloadAsset()]));
  document.getElementById('profileEskRefreshBtn')?.addEventListener('click', load);
  load().catch((error) => setMessage(error && error.message, true));
})();
