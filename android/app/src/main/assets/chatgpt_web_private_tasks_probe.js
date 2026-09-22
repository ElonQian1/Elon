(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateTasksProbe) {
    root.__elonChatGptPrivateTasksProbe = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  let core, busy = false;
  function handle(action, command, respond) {
    if (action !== 'private_protocol_probe' || command.value !== 'scheduled_tasks') return false;
    if (busy) { respond(action, false, 'tasks_busy'); return true; }
    busy = true;
    (async () => {
      core ||= page.__elonChatGptPrivateTasks.create(page);
      const listed = await core.run({ operation: 'list', filter: 'scheduled', force: true });
      if (!listed.ok) { respond(action, false, listed.code); return; }
      const cached = await core.run({ operation: 'list', filter: 'scheduled', force: false });
      const row = listed.data.items[0];
      const latest = row ? await core.run({ operation: 'latest', id: row.id, force: true }) : null;
      respond(action, true, JSON.stringify({ schema: 'elon.scheduled_tasks_probe.v1',
        catalogCount: listed.data.items.length, catalogComplete: listed.data.complete,
        cacheHit: cached.ok === true && cached.cached === true,
        latestState: latest === null ? 'not_sampled' : latest.ok ? latest.data.state : latest.code,
        updateCharacters: latest?.ok ? latest.data.update?.contentText?.length || 0 : 0,
        lastRunFailed: latest?.ok ? latest.data.lastRunFailed : null }));
    })().catch(() => respond(action, false, 'tasks_unavailable')).finally(() => { busy = false; });
    return true;
  }
  return Object.freeze({ version: 1, handle });
});
