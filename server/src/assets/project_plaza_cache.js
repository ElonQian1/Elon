(function () {
  function read(key) {
    try {
      const parsed = JSON.parse(window.localStorage.getItem(key) || 'null');
      if (!parsed || !Array.isArray(parsed.projects) || !parsed.projects.length) return null;
      const savedAt = Number(parsed.savedAt || 0);
      if (!Number.isFinite(savedAt) || savedAt <= 0) return null;
      return { projects: parsed.projects, savedAt };
    } catch (_) {
      return null;
    }
  }

  function write(key, snapshot) {
    try {
      window.localStorage.setItem(key, JSON.stringify({
        savedAt: snapshot.savedAt,
        projects: snapshot.projects
      }));
    } catch (_) {}
  }

  window.ElonProjectPlazaCache = Object.freeze({ read, write });
})();
