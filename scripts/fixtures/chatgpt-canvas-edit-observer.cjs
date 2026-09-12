'use strict';

// Behavioral fixture, not an invented export of the website's private edit store.
function observerFixture(edits = { userEdits: {}, timestamps: {} }) {
  let rendering = false, effects = [], mounts = 0, unmounts = 0;
  const rows = [], listeners = new Set(), document = { createElement: () => ({}) };
  const react = { createElement: component => component, useLayoutEffect: callback => {
    if (!rendering) throw Error('hook_outside_root');
    effects.push(callback);
  } };
  const dom = { flushSync: callback => { callback(); for (const effect of effects.splice(0)) effect(); } };
  const renderer = { createRoot: () => {
    mounts += 1;
    return { render: component => { rendering = true; try { component(); } finally { rendering = false; } },
      unmount: () => { unmounts += 1; effects = []; } };
  } };
  const conversation = { canvasDirtyInit() {}, useCanvasDirty: id => {
    if (!rendering) throw Error('hook_outside_root');
    const time = edits.timestamps[id];
    return (time?.lastTriggeredAt ?? 0) > (time?.lastFlushedAt ?? 0);
  } };
  const cache = { subscribe: listener => { listeners.add(listener); return () => listeners.delete(listener); },
    getAll: () => rows.concat(Object.entries(edits.userEdits).flatMap(([id, values]) => values.map(value => ({
    options: { mutationKey: ['canvas', 'textdoc', 'persist'] },
    state: { status: value.isPending === true ? 'pending' : value.isPending === false ? 'success' : 'unknown',
      variables: { textdocId: id, lastVersion: value.basedOnVersionInt ?? 3 },
      submittedAt: 1000, data: value.nextVersionInt ?? 4 }
  })))) };
  return { edits, rows, document, cache, conversation, react, dom, renderer,
    emit: event => { for (const listener of listeners) listener(event); }, listeners,
    runtime: { reactApi: () => react, reactDom: () => dom, reactRoot: () => renderer },
    client: { getMutationCache: () => cache }, counts: () => ({ mounts, unmounts }) };
}
module.exports = { observerFixture };
