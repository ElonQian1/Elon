(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && !root.__elonChatGptProjectHints) {
    root.__elonChatGptProjectHints = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const MAX_PROJECTS = 40;
  const PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  const PROJECT_PATH = /^\/g\/(g-p-[A-Za-z0-9_-]{1,160})(?:\/project)?$/;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function sanitize(values) {
    const projects = [];
    const seen = new Set();
    (Array.isArray(values) ? values : []).slice(0, MAX_PROJECTS).forEach((value) => {
      const path = cleanText(value && value.path);
      const match = path.match(PROJECT_PATH);
      const id = cleanText(value && value.id);
      const title = cleanText(value && value.title).slice(0, 160);
      if (!match || !PROJECT_ID.test(id) || match[1] !== id || !title || seen.has(path)) return;
      seen.add(path);
      projects.push({ id, title, path, active: value && value.active === true });
    });
    return projects;
  }

  function merge(observed, hinted) {
    const byPath = new Map();
    sanitize(hinted).forEach((project) => byPath.set(project.path, project));
    sanitize(observed).forEach((project) => byPath.set(project.path, project));
    return Array.from(byPath.values()).slice(0, MAX_PROJECTS);
  }

  function missingTitles(titles, projects) {
    const known = new Set(sanitize(projects).map((project) => cleanText(project.title).toLowerCase()));
    const seen = new Set();
    return (Array.isArray(titles) ? titles : []).map(cleanText).filter((title) => {
      const key = title.toLowerCase();
      return key && !known.has(key) && !seen.has(key) && seen.add(key);
    });
  }

  return Object.freeze({ sanitize, merge, missingTitles });
});
