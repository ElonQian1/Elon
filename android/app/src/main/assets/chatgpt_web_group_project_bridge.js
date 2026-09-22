(function (page) {
  'use strict';
  if (!page || page.location.origin !== 'https://chatgpt.com') return;
  let core;
  page.__elonChatGptGroupProjectBridge = Object.freeze({
    handle(action, command, respond) {
      if (action !== 'group_project_request') return false;
      let input;
      try { input = JSON.parse(command.value); }
      catch (_) { respond(action, false, JSON.stringify({ ok: false, code: 'project_input_invalid' })); return true; }
      core ||= page.__elonChatGptGroupProject.create(page);
      core.run(input).then(value => respond(action, value.ok, JSON.stringify(value)),
        () => respond(action, false, JSON.stringify({ ok: false, code: 'project_unavailable' })));
      return true;
    },
  });
})(typeof window === 'object' ? window : null);
