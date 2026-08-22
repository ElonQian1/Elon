(function (root, factory) {
  'use strict';

  const api = factory(root && root.__elonChatGptProjectPolicy);
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && !root.__elonChatGptProjectHints) {
    root.__elonChatGptProjectHints = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function (projectPolicy) {
  'use strict';

  const MAX_PROJECTS = 40;
  const PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  const PROJECT_PATH = /^\/g\/(g-p-[A-Za-z0-9_-]{1,160})(?:\/project)?$/;
  const PRODUCTION_PROJECT_ID = /^(g-p-[A-Fa-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?$/;
  const RESERVED_TITLE = /^(?:chat|chatgpt|\u804a\u5929|projects?|\u9879\u76ee|new project|create project|\u65b0\u5efa\u9879\u76ee|\u65b0\u9879\u76ee)$/i;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function canonicalId(value) {
    if (projectPolicy && typeof projectPolicy.projectId === 'function') {
      return projectPolicy.projectId(value);
    }
    const text = cleanText(value);
    const route = text.match(PROJECT_PATH);
    const id = route ? route[1] : text;
    if (!PROJECT_ID.test(id)) return '';
    const production = id.match(PRODUCTION_PROJECT_ID);
    return production ? production[1] : id;
  }

  function sanitize(values) {
    const projects = new Map();
    (Array.isArray(values) ? values : []).slice(0, MAX_PROJECTS).forEach((value) => {
      const path = cleanText(value && value.path);
      const match = path.match(PROJECT_PATH);
      const sourceId = cleanText(value && value.id);
      const id = sourceId ? canonicalId(sourceId) : canonicalId(path);
      const title = cleanText(value && value.title).slice(0, 160);
      if (!match || !id || canonicalId(match[1]) !== id || !title || RESERVED_TITLE.test(title)) return;
      projects.set(id, {
        id,
        title,
        path: '/g/' + id + '/project',
        active: value && value.active === true
      });
    });
    return Array.from(projects.values());
  }

  function merge(observed, hinted) {
    const byPath = new Map();
    sanitize(hinted).forEach((project) => byPath.set(project.id, project));
    sanitize(observed).forEach((project) => {
      if (!byPath.has(project.id)) byPath.set(project.id, project);
    });
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
