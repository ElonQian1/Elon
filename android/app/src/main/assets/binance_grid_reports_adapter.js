/* Source-indexed USD-M reports. Requests and credentials remain in the hosting document. */
(() => {
  'use strict';
  if (window !== window.top || location.origin !== 'https://www.binance.com') return;
  window.__elonBinanceReportsFactoryV1 = port => {
    const prefix = '/bapi/futures/';
    const routes = Object.freeze({
      history: ['POST', prefix + 'v2/private/future/grid/query-grid-history'],
      detail: ['GET', prefix + 'v1/private/future/grid/query-grid-detail'],
      windowOrders: ['GET', prefix + 'v2/private/future/grid/query-grid-open-items'],
      orders: ['POST', prefix + 'v1/private/future/strategy/streamer/um/open-orders'],
      matches: ['POST', prefix + 'v1/private/future/grid/query-grid-matched-items'],
      positions: ['POST', prefix + 'v1/private/future/strategy/user-data/get-future-user-positions']
    });
    const decimal = /^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/;
    const integer = /^(0|[1-9][0-9]{0,19})$/;
    const enumeration = /^[A-Z][A-Z0-9_]{0,63}$/;
    let serial = 0;
    const histories = new Map();
    const historyWindows = new Map();
    function scalar(v, pattern, required = false) {
      if (v == null && !required) return null;
      const text = typeof v === 'string' ? v : Number.isSafeInteger(v) ? String(v) : '';
      if (!pattern.test(text)) {
        if (!required) return null;
        throw Error('unsupported_field');
      }
      return text;
    }
    const dec = v => scalar(v, decimal);
    const num = v => scalar(v, integer);
    const en = v => scalar(v, enumeration);
    function array(value, max = 500) { if (!Array.isArray(value) || value.length > max) throw Error('unsupported_list'); return value; }
    function object(v) { if (!v || typeof v !== 'object' || Array.isArray(v)) throw Error('unsupported_object'); return v; }
    function history(v) {
      object(v);
      return {id: scalar(v.strategyId, integer, true), symbol: scalar(v.symbol, /^[A-Z0-9]{1,24}USDT$/, true),
        status: scalar(v.strategyStatus, enumeration, true), direction: en(v.direction),
        lower: dec(v.gridLowerLimit), upper: dec(v.gridUpperLimit), count: num(v.gridCount), leverage: num(v.initialLeverage),
        profit: dec(v.gridProfit), matchedPnl: dec(v.matchedPnl), fundingFee: dec(v.fundingFee), fee: dec(v.fee),
        created: num(v.createTime ?? v.bookTime), end: num(v.endTime), investment: dec(v.strategyAmount), initialNotional: dec(v.gridInitialValue)};
    }
    function order(v, side, windowed) {
      object(v);
      return {side: scalar(side ?? v.side, /^(BUY|SELL)$/, true), price: dec(v.price),
        quantity: dec(windowed ? v.qty : v.origQty), executed: dec(windowed ? null : v.executedQty),
        status: en(v.status), time: num(v.insertTime)};
    }
    function match(v) {
      object(v);
      return {sequence: scalar(v.matchedSeq, /^-?(0|[1-9][0-9]{0,19})$/, true), time: num(v.completionTime),
        profit: dec(v.profit), asset: en(v.profitAsset), cancelled: v.cancelEvent != null && v.cancelEvent !== false && v.cancelEvent !== '',
        details: array(v.details, 2).map(d => ({time: num(d.dealTime), side: scalar(d.side, /^(BUY|SELL)$/, true),
          type: en(d.orderType), price: dec(d.avgPrice), quantity: dec(d.executedQty), total: dec(d.totalAmount),
          fee: dec(d.feeAmount), feeAsset: en(d.feeAsset)}))};
    }
    async function request(key, params, context) {
      context.stage = key; context.http = 0; context.business = 'none';
      const [method, path] = routes[key];
      const url = path + (method === 'GET' ? '?strategyId=' + encodeURIComponent(params.strategyId) : '');
      const headers = new Headers(context.headers);
      if (method === 'POST') headers.set('content-type', 'application/json');
      const response = await port.fetch(url, {method, headers, credentials: 'same-origin', redirect: 'error',
        cache: 'no-store', signal: context.signal, ...(method === 'POST' ? {body: JSON.stringify(params)} : {})});
      const text = await response.text();
      context.http = response.status;
      if (response.status !== 200 || text.length > 1048576) throw Error('response_failed');
      const body = JSON.parse(text);
      context.business = typeof body.code === 'string' && /^[0-9]{6}$/.test(body.code) ? body.code : 'unknown';
      if (body.success !== true || body.code !== '000000') throw Error('business_failed');
      context.stage = 'parse_' + key;
      return body;
    }
    function valid(q) {
      if (!q || Object.keys(q).sort().join(',') !== 'days,id,kind,page,request,symbol') return false;
      if (!/^[0-9a-f]{32}$/.test(q.request) || !['history','orders','matches','positions'].includes(q.kind)) return false;
      if (!Number.isInteger(q.page) || q.page < 1 || q.page > 1000 || ![7,30,90].includes(q.days)) return false;
      if (q.kind === 'history') return q.id === '' && (q.symbol === '' || /^[A-Z0-9]{1,24}USDT$/.test(q.symbol)) &&
        (q.page === 1 || historyWindows.has(q.days + ':' + q.symbol));
      return /^[0-9]{1,20}$/.test(q.id) && /^[A-Z0-9]{1,24}USDT$/.test(q.symbol) &&
        (port.known(q.id) || histories.get(q.id) === q.symbol) && (q.kind === 'matches' || q.page === 1);
    }
    return Object.freeze({
      reset() { serial++; histories.clear(); historyWindows.clear(); },
      query(q) {
        const captured = port.context();
        if (!valid(q) || !captured) return false;
        const mine = ++serial;
        const controller = new AbortController(), timer = setTimeout(() => controller.abort(), 25000);
        const context = {...captured, signal: controller.signal, stage:'identity_before', http:0, business:'none'};
        const diagnostic = (outcome, error='none') => window.__elonBinanceDiagnosticsV1?.report({
          kind:q.kind, stage:context.stage, outcome, error, http:context.http, business:context.business});
        diagnostic('loading');
        const still = () => mine === serial && port.context()?.account === context.account;
        (async () => {
          const before = await port.prove(context.headers);
          if (!before || before.account !== context.account || !still()) throw Error('account_changed');
          let rows, total, coverage = 'page';
          if (q.kind === 'history') {
            const windowKey = q.days + ':' + q.symbol;
            if(q.page === 1) {
              if(historyWindows.size >= 32) historyWindows.clear();
              historyWindows.set(windowKey, Date.now());
            }
            const endTime = historyWindows.get(windowKey), startTime = endTime - q.days * 86400000;
            if(!Number.isSafeInteger(endTime)) throw Error('restart_pagination');
            const response = await request('history', {page:q.page,rows:20,startTime,endTime,...q.symbol?{symbol:q.symbol}:{}}, context);
            const rawRows = array(object(response.data).grids, 20);
            if (rawRows.some(v => v.rootUserId != null && scalar(v.rootUserId, integer, true) !== before.account)) throw Error('account_changed');
            rows = rawRows.map(history); total = Number(scalar(response.data.total, integer, true));
          } else {
            const detail = object((await request('detail', {strategyId:q.id}, context)).data);
            if (scalar(detail.strategyId, integer, true) !== q.id || detail.symbol !== q.symbol ||
                (detail.rootUserId != null && scalar(detail.rootUserId, integer, true) !== before.account)) throw Error('scope_changed');
            if (q.kind === 'matches') {
              const response = await request('matches', {strategyId:q.id,page:q.page,rows:20}, context);
              rows = array(response.data, 20).map(match); total = Number(scalar(response.total, integer, true));
            } else if (q.kind === 'orders') {
              coverage = detail.slideWindow === true ? 'grid_slots' : 'open_orders';
              if (detail.slideWindow === true) {
                const data = object((await request('windowOrders', {strategyId:q.id}, context)).data);
                rows = [...array(data.bidItems).map(v => order(v, 'BUY', true)), ...array(data.askItems).map(v => order(v, 'SELL', true))];
              } else {
                const data = array((await request('orders', {}, context)).data, 10000);
                rows = data.filter(v => scalar(object(v).strategyId, integer, true) === q.id).map(v => {
                  if (v.symbol !== q.symbol) throw Error('scope_changed');
                  return order(v, null, false);
                });
              }
              total = rows.length;
              if (rows.length > 500) { rows = rows.slice(0,500); coverage = 'limited_' + coverage; }
            } else {
              coverage = 'strategy_position';
              const uid = scalar(detail.strategyUserId, integer, true);
              const data = object((await request('positions', {}, context)).data);
              // Strategy shadow UID, never parent UID or the consumer supplied account.
              rows = array(data[uid] ?? []).filter(v => v.symbol === q.symbol && v.positionSide === 'BOTH').map(v => ({
                symbol:q.symbol, quantity:dec(v.positionAmount), entry:dec(v.entryPrice), isolatedWallet:dec(v.isolatedWallet),
                isolated: typeof v.isolated === 'boolean' ? v.isolated : null
              }));
              total = rows.length;
            }
          }
          if (!Number.isSafeInteger(total) || total < rows.length || total > 10000000) throw Error('unsupported_total');
          context.stage = 'identity_after';
          const after = await port.prove(context.headers);
          if (!after || after.account !== before.account || after.account_kind !== before.account_kind || !still()) throw Error('account_changed');
          if (q.kind === 'history') {
            if (histories.size + rows.length > 2000) histories.clear();
            rows.forEach(v => histories.set(v.id, v.symbol));
          }
          port.emit({schema:'yilong.binance_report_observation.v1',request:q.request,kind:q.kind,
            account:after.account,account_kind:after.account_kind,status:'ready',page:q.page,total,coverage,rows});
          diagnostic('ready');
        })().catch(error => {
          if (still()) diagnostic('failed', error?.message);
          if (still()) port.emit({schema:'yilong.binance_report_observation.v1',request:q.request,kind:q.kind,
            account:context.account,account_kind:beforeKind(context),status:'error',page:q.page,total:0,coverage:'unavailable',rows:[]});
        }).finally(() => clearTimeout(timer));
        return true;
      }
    });
    function beforeKind(context) { return context.account_kind; }
  };
})();
