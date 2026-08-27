(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && (!root.__elonGoogleWebPrivateReplyReconciler ||
      Number(root.__elonGoogleWebPrivateReplyReconciler.version || 0) < api.version)) {
    root.__elonGoogleWebPrivateReplyReconciler = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const completedAnswers = new Set();
  const promptBaselineAnswers = new Set();
  let observedPrompt = '';
  let observedLocationKey = '';

  function cleanText(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
  }

  function contentText(message) {
    const content = Array.isArray(message && message.content) ? message.content : [];
    return cleanText(content.map((part) => {
      if (typeof part === 'string') return part;
      return part && (part.type === 'text' || part.type === 'markdown') ? part.text : '';
    }).filter(Boolean).join(' '));
  }

  function matchingUserIndex(messages, prompt) {
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      if (messages[index] && messages[index].role === 'user' &&
          contentText(messages[index]) === prompt) return index;
    }
    return -1;
  }

  function privateMessage(userIndex, reply) {
    return {
      id: 'google-private-answer-' + userIndex,
      role: 'assistant',
      state: reply.streaming ? 'streaming' : 'completed',
      content: [{ type: 'text', text: cleanText(reply.text) }]
    };
  }

  function privateUserMessage(index, prompt) {
    return {
      id: 'google-private-prompt-' + index,
      role: 'user',
      state: 'completed',
      content: [{ type: 'text', text: prompt }]
    };
  }

  function locationKey(value) {
    try {
      const url = new URL(String(value || ''));
      const conversationId = cleanText(url.searchParams.get('csuir'));
      if (conversationId) return url.origin + url.pathname + '|csuir=' + conversationId;
      return url.origin + url.pathname + '|q=' + cleanText(url.searchParams.get('q')) +
        '&udm=' + cleanText(url.searchParams.get('udm')) +
        '&aep=' + cleanText(url.searchParams.get('aep'));
    } catch (_) {
      return '';
    }
  }

  function observePrompt(messages, prompt, href) {
    observedPrompt = cleanText(prompt);
    observedLocationKey = locationKey(href);
    promptBaselineAnswers.clear();
    if (!Array.isArray(messages) || !observedPrompt) return;
    messages
      .filter((message) => message && message.role === 'assistant')
      .map(contentText)
      .filter(Boolean)
      .forEach((value) => promptBaselineAnswers.add(value));
  }

  function apply(messages, reply, href) {
    if (!Array.isArray(messages) || !reply) return false;
    const prompt = cleanText(reply.prompt);
    const answer = cleanText(reply.text);
    if (!prompt || !answer) return false;
    const userIndex = matchingUserIndex(messages, prompt);
    if (userIndex < 0) {
      if (prompt !== observedPrompt || !observedLocationKey ||
          locationKey(href) !== observedLocationKey) return false;
      const syntheticUserIndex = messages.length;
      messages.push(privateUserMessage(syntheticUserIndex, prompt));
      messages.push(privateMessage(syntheticUserIndex, reply));
      if (!reply.streaming) completedAnswers.add(answer);
      return true;
    }

    const earlierAnswers = new Set(completedAnswers);
    if (prompt === observedPrompt) {
      promptBaselineAnswers.forEach((value) => earlierAnswers.add(value));
    }
    messages.slice(0, userIndex)
      .filter((message) => message && message.role === 'assistant')
      .map(contentText)
      .filter(Boolean)
      .forEach((value) => earlierAnswers.add(value));
    const assistants = [];
    for (let index = userIndex + 1; index < messages.length; index += 1) {
      if (messages[index] && messages[index].role === 'assistant') assistants.push(index);
    }
    if (!assistants.length) {
      messages.splice(userIndex + 1, 0, privateMessage(userIndex, reply));
      if (!reply.streaming) completedAnswers.add(answer);
      return true;
    }

    const staleAssistants = assistants.filter((index) =>
      earlierAnswers.has(contentText(messages[index]))
    );
    if (staleAssistants.length !== assistants.length) {
      if (!reply.streaming) completedAnswers.add(answer);
      return false;
    }

    for (let index = assistants.length - 1; index >= 0; index -= 1) {
      messages.splice(assistants[index], 1);
    }
    messages.splice(userIndex + 1, 0, privateMessage(userIndex, reply));
    if (!reply.streaming) completedAnswers.add(answer);
    return true;
  }

  return Object.freeze({
    version: 2,
    cleanText,
    contentText,
    locationKey,
    observePrompt,
    apply
  });
});
