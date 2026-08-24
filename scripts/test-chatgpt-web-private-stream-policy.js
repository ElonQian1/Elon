'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');

const policy = require(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_stream_policy.js'
));

const payload = (text, status = 'in_progress') => ({
  conversation_id: 'conversation-one',
  message: {
    id: 'assistant-one',
    author: { role: 'assistant' },
    status,
    content: { parts: [text] }
  }
});

assert.deepEqual(policy.assistantFrame(payload('first')), {
  id: 'assistant-one',
  conversationId: 'conversation-one',
  text: 'first',
  citations: [],
  state: 'streaming'
});
assert.equal(policy.assistantFrame({ message: { author: { role: 'user' } } }), null);
assert.equal(policy.assistantFrame({
  message: {
    id: 'assistant-object-parts',
    author: { role: 'assistant' },
    content: { parts: [{ text: 'first' }, { content: 'second' }] }
  }
}).text, 'first\nsecond');
assert.deepEqual(policy.progressFrame({
  c: 1,
  v: {
    conversation_id: 'conversation-one',
    message: {
      author: { role: 'assistant' },
      content: { content_type: 'code', parts: [{ text: 'private tool instruction' }] },
      metadata: { reasoning_title: '正在搜索 South Korea stock market' }
    }
  }
}), {
  conversationId: 'conversation-one',
  progressLabel: '正在搜索 South Korea stock market',
  state: 'streaming'
});

const decoded = [];
let done = 0;
const decoder = policy.createSseDecoder((value) => decoded.push(value), () => { done += 1; });
decoder.push('event: message\ndata: {"message":{"author":{"role":"assistant"},');
decoder.push('"content":{"parts":["hel');
decoder.push('lo"]}}}\n\ndata: [DONE]\n\n');
assert.equal(decoded.length, 1);
assert.equal(decoded[0].message.content.parts[0], 'hello');
assert.equal(done, 1);

let now = 1000;
const session = policy.createSession({ now: () => now });
session.begin();
assert.equal(session.accept(payload('hello')), true);
assert.equal(session.current('/c/conversation-one').state, 'streaming');
assert.equal(session.current('/c/another-conversation'), null);

const user = { id: 'user-one', role: 'user', state: 'completed', content: [{ type: 'text', text: 'question' }] };
let merged = session.merge([user], '/c/conversation-one');
assert.equal(merged.length, 2);
assert.equal(merged[1].id, 'private-stream:assistant-one');
assert.equal(merged[1].content[0].text, 'hello');
assert.equal(merged[1].state, 'streaming');

assert.equal(session.accept(payload('hello world', 'finished_successfully')), true);
assert.equal(session.finish(), true);
merged = session.merge([user, {
  id: 'assistant-one',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'hello' }, { type: 'citation', text: 'source' }]
}], '/c/conversation-one');
assert.equal(merged.length, 2);
assert.equal(merged[1].content[0].text, 'hello world');
assert.equal(merged[1].content[1].type, 'citation');

const longerDom = policy.mergeMessages([{
  id: 'assistant-one',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'hello world from DOM' }]
}], {
  id: 'assistant-one',
  text: 'hello world',
  state: 'streaming'
});
assert.equal(longerDom[0].content[0].text, 'hello world from DOM');
assert.equal(longerDom[0].state, 'streaming');

merged = session.merge([user, {
  id: 'assistant-one',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'hello world' }]
}], '/c/conversation-one');
assert.equal(merged.length, 2);
assert.equal(session.current('/c/conversation-one').state, 'completed');

const positionalMerged = policy.mergeMessages([user, {
  id: 'conversation-turn-2',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'hello world [Reuters](https://reuters.com/example)' }]
}], {
  id: 'assistant-private-id',
  text: 'hello world',
  state: 'completed'
});
assert.equal(positionalMerged.length, 2, 'the DOM and private stream for the current turn are merged');
assert.equal(positionalMerged[1].id, 'conversation-turn-2');

const nextTurnMerged = policy.mergeMessages([{
  id: 'assistant-previous',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'previous answer' }]
}, user], {
  id: 'assistant-current',
  text: 'current answer',
  state: 'streaming'
});
assert.equal(nextTurnMerged.length, 3, 'an assistant before the latest user is not overwritten');
assert.equal(nextTurnMerged[2].id, 'private-stream:assistant-current');

const financeWidget = {
  default_range: '1D',
  timeframe_order: ['1D', '5D'],
  timeframe_configs: {
    '1D': {
      chart: {
        data: [
          { timestamp: 1, close: 77000, formatted: '12:00 上午' },
          { timestamp: 2, close: 77100, formatted: '12:05 上午' }
        ]
      },
      summary: {
        price_text: 'US$77,100.00',
        price_change_text: '+US$100.00 (0.13%)',
        price_change_color: 'success'
      }
    },
    '5D': { chart: { data: [] }, summary: {} }
  },
  asset_display_name: 'Bitcoin (BTC)',
  current_price_text: 'US$77,100.00',
  metrics_display: [{ cols: [
    { label: '当日最低价', value: '75,853' },
    { label: '当日最高价', value: '78,003' }
  ] }]
};
const financePart = policy.financePartFromWidget(financeWidget);
assert.equal(financePart.type, 'rich_card');
assert.equal(financePart.richContent.source, 'private_response');
assert.equal(financePart.richContent.payload.symbol, 'BTC');
assert.equal(financePart.richContent.payload.chart.kind, 'line');
assert.equal(financePart.richContent.payload.chart.points.length, 2);
assert.equal(financePart.richContent.payload.periods[0].selected, true);
assert.equal(financePart.richContent.payload.metrics.length, 2);

const chartMetadata = {
  content_references: [{
    type: 'client_defined_widget',
    category: 'visualization',
    data: {
      language: 'recharts-json',
      widget_type: 'charts_widget_v2',
      content: {
        chartType: 'line',
        meta: { title: '比特币近期走势', description: 'BTC/USD 日内收盘价参考。' },
        xKey: 'date',
        series: [{ dataKey: 'price', label: 'BTC/USD', valuePrefix: '$' }],
        data: [
          { date: '8/20', price: 69268 },
          { date: '8/21', price: 73031 },
          { date: '8/22', price: 78333 },
          { date: '8/24', price: 77728 }
        ]
      }
    }
  }]
};
const clientChart = policy.clientChartPartFromMetadata(chartMetadata);
assert.equal(clientChart.type, 'rich_card');
assert.equal(clientChart.richContent.kind, 'chart');
assert.equal(clientChart.richContent.payload.series[0].label, 'BTC/USD');
assert.equal(clientChart.richContent.payload.points.length, 4);
assert.equal(clientChart.richContent.payload.points[3].values[0], 77728);
assert.equal(policy.assistantFrame(payload(
  '正文前\ue200genui\ue202{"chart":{"content":{}}}\ue201正文后'
)).text, '正文前正文后');

const richSession = policy.createSession({ now: () => 3000 });
richSession.begin();
richSession.accept(payload('finance answer', 'finished_successfully'));
assert.equal(richSession.acceptRichParts([financePart], {
  messageId: 'assistant-one',
  conversationId: 'conversation-one'
}), true);
const richMerged = richSession.merge([user, {
  id: 'conversation-turn-finance',
  role: 'assistant',
  state: 'completed',
  content: [
    { type: 'markdown', text: 'finance answer' },
    { type: 'interactive', text: '交互内容', kind: 'interactive' }
  ]
}], '/c/conversation-one');
assert.equal(richMerged.length, 2);
assert.equal(richMerged[1].content.filter((part) => part.type === 'rich_card').length, 1);
assert.equal(richMerged[1].content.some((part) => part.type === 'interactive'), false);

session.begin();
session.accept(payload('stale'));
now += 5 * 60 * 1000 + 1;
assert.equal(session.current('/c/conversation-one'), null);

const compactSession = policy.createSession({ now: () => 2000 });
compactSession.begin();
assert.equal(compactSession.accept({
  c: 1,
  v: {
    conversation_id: 'conversation-compact',
    message: {
      id: 'assistant-search',
      author: { role: 'assistant' },
      status: 'finished_successfully',
      content: { content_type: 'code', parts: [{ text: 'private tool instruction' }] },
      metadata: { reasoning_title: '正在搜索 South Korea stock market' }
    }
  }
}), true);
assert.equal(
  compactSession.current('/c/conversation-compact').progressLabel,
  '正在搜索 South Korea stock market'
);
assert.equal(compactSession.accept({
  c: 13,
  v: {
    conversation_id: 'conversation-compact',
    message: {
      id: 'assistant-compact',
      author: { role: 'assistant' },
      status: 'in_progress',
      content: { content_type: 'text', parts: [''] },
      metadata: { content_references: [] }
    }
  }
}), true);
const compactPlaceholder = compactSession.merge([], '/c/conversation-compact');
assert.equal(compactPlaceholder.length, 1);
assert.equal(compactPlaceholder[0].state, 'streaming');
assert.equal(compactPlaceholder[0].content[0].text, '');
assert.equal(compactSession.accept({
  o: 'append',
  p: '/message/content/parts/0',
  v: 'KOSPI opened higher'
}), true);
assert.equal(compactSession.accept({ v: ' and remained volatile.' }), true);
assert.equal(
  compactSession.current('/c/conversation-compact').text,
  'KOSPI opened higher and remained volatile.'
);

const chartSession = policy.createSession({ now: () => 2500 });
chartSession.begin();
assert.equal(chartSession.accept({
  c: 17,
  v: {
    conversation_id: 'conversation-chart',
    message: {
      id: 'assistant-chart',
      author: { role: 'assistant' },
      status: 'in_progress',
      content: { content_type: 'text', parts: ['走势怎么判断\n\n\ue200genui\ue202{"chart":'] },
      metadata: { content_references: [{
        type: 'client_defined_widget',
        category: 'visualization',
        data: {
          language: 'recharts-json',
          widget_type: 'charts_widget_v2',
          content: {
            chartType: 'line',
            meta: { title: '比特币近期走势', description: 'BTC/USD' },
            xKey: 'date',
            series: [{ dataKey: 'price', label: 'BTC/USD', valuePrefix: '$' }],
            data: [{ date: '8/20' }]
          }
        }
      }] }
    }
  }
}), true);
assert.equal(chartSession.accept({
  v: [
    { p: '/message/metadata/content_references/0/data/content/data/0/price', o: 'add', v: 69268 },
    { p: '/message/metadata/content_references/0/data/content/data', o: 'append', v: [
      { date: '8/21', price: 73031 }, { date: '8/24', price: 77728 }
    ] },
    { p: '/message/content/parts/0', o: 'append', v: '}\ue201\n\n短线偏强。' },
    { p: '/message/status', o: 'replace', v: 'finished_successfully' }
  ]
}), true);
const chartCurrent = chartSession.current('/c/conversation-chart');
assert.equal(chartCurrent.text, '走势怎么判断\n\n短线偏强。');
assert.equal(chartCurrent.richParts.length, 1);
assert.equal(chartCurrent.richParts[0].richContent.kind, 'chart');
assert.equal(chartCurrent.richParts[0].richContent.payload.points.length, 3);
assert.equal(compactSession.accept({
  o: 'patch',
  p: '',
  v: [
    { o: 'append', p: '/message/content/parts/0', v: ' Final.' },
    { o: 'replace', p: '/message/status', v: 'finished_successfully' }
  ]
}), true);
assert.equal(compactSession.current('/c/conversation-compact').state, 'completed');
assert.equal(
  compactSession.current('/c/conversation-compact').text,
  'KOSPI opened higher and remained volatile. Final.'
);

console.log('CHATGPT_WEB_PRIVATE_STREAM_POLICY_TESTS=passed');
