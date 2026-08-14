(function () {
  'use strict';

  if (window.__elonChatGptConversations || location.origin !== 'https://chatgpt.com') return;

  const MAX_CONVERSATIONS = 100;
  const MAX_PROJECTS = 40;
  const projectPolicy = window.__elonChatGptProjectPolicy;
  const CONVERSATION_PATH = /^(?:\/c\/[A-Za-z0-9_-]{1,160}|\/g\/(g-p-[A-Za-z0-9_-]{1,160})\/c\/[A-Za-z0-9_-]{1,160})$/;
  const PROJECT_PATH = /^\/g\/(g-p-[A-Za-z0-9_-]{1,160})(?:\/project)?$/;
  const GROUP_LABEL = /^(?:today|yesterday|previous \d+ days|last \d+ days|older|今天|昨天|前 ?\d+ ?天|过去 ?\d+ ?天|更早)$/i;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function isVisible(node) {
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 &&
      rect.bottom > 0 && rect.right > 0 && rect.top < window.innerHeight && rect.left < window.innerWidth &&
      style.display !== 'none' && style.visibility !== 'hidden';
  }

  function label(node) {
    return cleanText([
      node && node.getAttribute('aria-label'),
      node && node.getAttribute('title'),
      node && node.textContent
    ].filter(Boolean).join(' ')).toLowerCase();
  }

  function conversationPath(node) {
    try {
      const url = new URL(node.getAttribute('href') || '', location.origin);
      return url.origin === location.origin && CONVERSATION_PATH.test(url.pathname)
        ? url.pathname
        : '';
    } catch {
      return '';
    }
  }

  function sameOriginPath(node) {
    try {
      const url = new URL(node.getAttribute('href') || '', location.origin);
      return url.origin === location.origin ? url.pathname : '';
    } catch {
      return '';
    }
  }

  function projectIdFromPath(path) {
    const match = String(path || '').match(CONVERSATION_PATH) || String(path || '').match(PROJECT_PATH);
    return match && match[1] ? match[1] : '';
  }

  function readProjects() {
    if (projectPolicy && typeof projectPolicy.read === 'function') {
      return projectPolicy.read(document, isVisible, (node) => cleanText(
        node.getAttribute('data-project-title') ||
          node.getAttribute('title') ||
          node.getAttribute('aria-label') ||
          node.textContent
      )).map(({ id, title, path }) => ({
        id,
        title,
        path,
        active: location.pathname.startsWith('/g/' + id + '/')
      })).slice(0, MAX_PROJECTS);
    }
    const seen = new Set();
    return Array.from(document.querySelectorAll('a[href*="/g/g-p-"]')).map((node) => {
      const path = sameOriginPath(node);
      const match = path.match(PROJECT_PATH);
      if (!match || seen.has(path)) return null;
      const title = cleanText(
        node.getAttribute('data-project-title') ||
          node.getAttribute('title') ||
          node.getAttribute('aria-label') ||
          node.textContent
      ).slice(0, 160);
      if (!title) return null;
      seen.add(path);
      return {
        id: match[1],
        title,
        path,
        active: location.pathname.startsWith('/g/' + match[1] + '/')
      };
    }).filter(Boolean).slice(0, MAX_PROJECTS);
  }

  function projectLabel(node) {
    return cleanText(
      node.getAttribute('data-project-title') ||
        node.getAttribute('title') ||
        node.getAttribute('aria-label') ||
        node.textContent
    );
  }

  function enrichProjectConversations(conversations, projects) {
    const byId = new Map(projects.map((project) => [project.id, project]));
    return conversations.map((conversation) => {
      const project = byId.get(conversation.projectId);
      if (!project) return conversation;
      return Object.assign({}, conversation, {
        projectTitle: project.title,
        projectPath: project.path
      });
    });
  }

  function groupLabelFor(node) {
    const nodeTop = node.getBoundingClientRect().top;
    let scope = node.parentElement;
    for (let depth = 0; scope && depth < 7; depth += 1, scope = scope.parentElement) {
      const candidates = Array.from(scope.children).filter((candidate) => {
        if (candidate === node || candidate.contains(node)) return false;
        const text = cleanText(candidate.textContent || candidate.getAttribute('aria-label'));
        if (!text || text.length > 80 || !GROUP_LABEL.test(text)) return false;
        return candidate.getBoundingClientRect().top <= nodeTop + 1;
      });
      const nearest = candidates.sort((left, right) =>
        right.getBoundingClientRect().top - left.getBoundingClientRect().top
      )[0];
      if (nearest) return cleanText(nearest.textContent || nearest.getAttribute('aria-label')).slice(0, 80);
    }
    return '';
  }

  function localIsoDate(date) {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return year + '-' + month + '-' + day;
  }

  function activityDateFor(node, groupLabel) {
    const time = node.querySelector('time[datetime]') ||
      (node.parentElement && node.parentElement.querySelector('time[datetime]'));
    const parsedTime = time && new Date(time.getAttribute('datetime'));
    if (parsedTime && !Number.isNaN(parsedTime.getTime())) return localIsoDate(parsedTime);
    const normalized = cleanText(groupLabel).toLowerCase();
    if (/^(?:today|今天)$/.test(normalized)) return localIsoDate(new Date());
    if (/^(?:yesterday|昨天)$/.test(normalized)) {
      const yesterday = new Date();
      yesterday.setDate(yesterday.getDate() - 1);
      return localIsoDate(yesterday);
    }
    return '';
  }

  function conversationLinks() {
    return Array.from(document.querySelectorAll('a[href*="/c/"]'))
      .filter((node) => conversationPath(node));
  }

  function readConversations() {
    const seen = new Set();
    return conversationLinks().map((node) => {
      const path = conversationPath(node);
      if (!path || seen.has(path)) return null;
      seen.add(path);
      const titleNode = node.querySelector('[title], [dir="auto"], span') || node;
      const title = cleanText(
        node.getAttribute('data-conversation-title')
          || titleNode.getAttribute('title')
          || titleNode.textContent
          || node.getAttribute('aria-label')
      ).slice(0, 160);
      if (!title) return null;
      const projectId = projectIdFromPath(path);
      const groupLabel = groupLabelFor(node);
      return {
        id: path.split('/').filter(Boolean).pop() || path,
        title,
        path,
        active: path === location.pathname || node.getAttribute('aria-current') === 'page',
        groupLabel,
        projectId: projectId || null,
        projectTitle: null,
        projectPath: projectId ? '/g/' + projectId + '/project' : null,
        activityDates: [activityDateFor(node, groupLabel)].filter(Boolean)
      };
    }).filter(Boolean).slice(0, MAX_CONVERSATIONS);
  }

  function findConversationScroller() {
    const links = conversationLinks();
    const candidates = [];
    links.slice(0, 5).forEach((link) => {
      let node = link.parentElement;
      while (node && node !== document.body && node !== document.documentElement) {
        if (!candidates.includes(node)) candidates.push(node);
        node = node.parentElement;
      }
    });
    return candidates.find((node) => {
      const style = window.getComputedStyle(node);
      return Number(node.clientHeight) >= 80 &&
        Number(node.scrollHeight) > Number(node.clientHeight) + 24 &&
        /auto|scroll/.test(style.overflowY || '');
    }) || candidates.find((node) =>
      Number(node.clientHeight) >= 80 &&
      Number(node.scrollHeight) > Number(node.clientHeight) + 24
    ) || null;
  }

  function collectConversationHistory(initial, onDone) {
    const history = window.__elonChatGptConversationHistory;
    if (!history || typeof history.collect !== 'function') {
      return onDone({
        conversations: initial,
        collection: {
          scrollerFound: false,
          scrolled: false,
          scrollRestored: true,
          reachedEnd: false,
          truncated: initial.length >= MAX_CONVERSATIONS,
          timedOut: false,
          observedCount: initial.length,
          steps: 0
        }
      });
    }
    history.collect({
      initial,
      read: readConversations,
      findScroller: findConversationScroller,
      maximum: MAX_CONVERSATIONS,
      timeoutMs: 10000,
      delayMs: 180,
      maxSteps: 40,
      stablePasses: 3
    }, onDone);
  }

  function collectProjectHistory(initial, onDone) {
    const history = window.__elonChatGptConversationHistory;
    if (!history || typeof history.collect !== 'function') return onDone(initial);
    history.collect({
      initial,
      read: readProjects,
      findScroller: findConversationScroller,
      maximum: MAX_PROJECTS,
      timeoutMs: 10000,
      delayMs: 180,
      maxSteps: 40,
      stablePasses: 2
    }, (snapshot) => onDone(snapshot.conversations));
  }

  function findSidebarButton(open) {
    const selector = open
      ? '[data-testid*="open-sidebar" i], button[aria-label*="open sidebar" i], button[aria-label*="打开边栏" i], button[aria-label*="打开侧边栏" i]'
      : '[data-testid*="close-sidebar" i], button[aria-label*="close sidebar" i], button[aria-label*="关闭边栏" i], button[aria-label*="关闭侧边栏" i]';
    const direct = document.querySelector(selector);
    if (direct && isVisible(direct)) return direct;
    const needles = open
      ? ['open sidebar', '打开边栏', '打开侧边栏']
      : ['close sidebar', '关闭边栏', '关闭侧边栏'];
    return Array.from(document.querySelectorAll('button')).find((button) =>
      isVisible(button) && needles.some((needle) => label(button).includes(needle))
    ) || null;
  }

  function findNewConversationNode() {
    const stableControl = document.querySelector(
      '[data-testid="create-new-chat-button"], [data-testid="new-chat-button"]'
    );
    if (stableControl && isVisible(stableControl)) return stableControl;

    return Array.from(document.querySelectorAll(
      'a[href="/"], [data-testid*="new-chat" i], [data-testid*="create-new-chat" i], ' +
      'button, [role="button"], [role="link"]'
    )).find((node) => {
      if (!isVisible(node)) return false;
      return /new chat|create chat|new conversation|新聊天|新建聊天|新建会话/.test(label(node));
    }) || null;
  }

  function waitForNewConversation(onReady, onTimeout) {
    const started = Date.now();
    function poll() {
      const target = findNewConversationNode();
      if (target) return onReady(target);
      if (Date.now() - started >= 3000) return onTimeout();
      window.setTimeout(poll, 100);
    }
    poll();
  }

  function waitForConversations(onReady, onTimeout) {
    const started = Date.now();
    let stableSince = started;
    let lastFingerprint = '';
    let best = [];
    function poll() {
      const conversations = readConversations();
      if (conversations.length > best.length) best = conversations;
      const fingerprint = conversations.map((conversation) => conversation.path).join('|');
      if (fingerprint !== lastFingerprint) {
        lastFingerprint = fingerprint;
        stableSince = Date.now();
      }
      if (conversations.length && Date.now() - stableSince >= 500) return onReady(best);
      if (Date.now() - started >= 3000) {
        return best.length ? onReady(best) : onTimeout();
      }
      window.setTimeout(poll, 100);
    }
    poll();
  }

  function waitForRoute(predicate, onReady, onTimeout) {
    const started = Date.now();
    function poll() {
      if (predicate(location.pathname)) return onReady(location.pathname);
      if (Date.now() - started >= 3000) return onTimeout();
      window.setTimeout(poll, 80);
    }
    poll();
  }

  function collectProjects(initial, onDone) {
    if (!projectPolicy || typeof projectPolicy.unresolved !== 'function') return onDone(initial);
    const originalPath = location.pathname;
    const titles = projectPolicy.unresolved(document, isVisible, projectLabel)
      .map((project) => project.title);
    const values = initial.slice();
    const seen = new Set(values.map((project) => project.id));
    let index = 0;

    function ensureSidebar(next) {
      if (projectPolicy.unresolved(document, isVisible, projectLabel).length) return next();
      const open = findSidebarButton(true);
      if (!open) return onDone(values);
      open.click();
      const started = Date.now();
      function poll() {
        if (projectPolicy.unresolved(document, isVisible, projectLabel).length) return next();
        if (Date.now() - started >= 3000) return onDone(values);
        window.setTimeout(poll, 80);
      }
      poll();
    }

    function restore(next) {
      if (location.pathname === originalPath) return ensureSidebar(next);
      history.back();
      waitForRoute(
        (path) => path === originalPath,
        () => ensureSidebar(next),
        () => onDone(values)
      );
    }

    function visitNext() {
      if (index >= titles.length) return restore(() => onDone(values));
      const title = titles[index++];
      const candidate = projectPolicy.unresolved(document, isVisible, projectLabel)
        .find((project) => project.title === title);
      if (!candidate) return visitNext();
      const before = location.pathname;
      try {
        candidate.node.click();
      } catch {
        return restore(visitNext);
      }
      waitForRoute(
        (path) => path !== before && /^\/g\/g-p-[A-Za-z0-9_-]{1,160}\/project$/.test(path),
        (path) => {
          const id = projectIdFromPath(path);
          if (id && !seen.has(id)) {
            seen.add(id);
            values.push({ id, title, path, active: originalPath.startsWith('/g/' + id + '/') });
          }
          restore(visitNext);
        },
        () => restore(visitNext)
      );
    }

    visitNext();
  }

  function emitConversationSnapshot(snapshot, emitEvent, result, closeAfter) {
    collectProjectHistory(readProjects(), (observedProjects) => {
      collectProjects(observedProjects, (projects) => {
        emitEvent({
          type: 'conversation_snapshot',
          conversations: enrichProjectConversations(snapshot.conversations, projects),
          projects,
          collection: snapshot.collection
        });
        result('list_conversations', true, '');
        if (closeAfter) {
          const close = findSidebarButton(false);
          if (close) close.click();
        }
      });
    });
  }

  function requestList(emitEvent, result) {
    const open = findSidebarButton(true);
    if (open) {
      open.click();
      return waitForConversations(
        (conversations) => {
          collectConversationHistory(conversations, (snapshot) => {
            emitConversationSnapshot(snapshot, emitEvent, result, true);
          });
        },
        () => result('list_conversations', false, '官网会话列表尚未加载完成。')
      );
    }

    const existing = readConversations();
    if (!existing.length) return result('list_conversations', false, '未找到官网会话侧栏入口。');
    collectConversationHistory(existing, (snapshot) => {
      emitConversationSnapshot(snapshot, emitEvent, result, false);
    });
  }

  function newConversation(result) {
    if (location.pathname === '/') return result('new_conversation', true, '');
    const existing = findNewConversationNode();
    if (existing) {
      result('new_conversation', true, '');
      existing.click();
      return;
    }

    const open = findSidebarButton(true);
    if (!open) return result('new_conversation', false, '未找到新建会话入口。');
    open.click();
    waitForNewConversation(
      (target) => {
        result('new_conversation', true, '');
        target.click();
      },
      () => result('new_conversation', false, '官网新建会话入口尚未加载完成。')
    );
  }

  function openConversation(path, result) {
    if (!CONVERSATION_PATH.test(path)) {
      return result('open_conversation', false, '会话地址无效。');
    }
    const target = conversationLinks().find((node) => conversationPath(node) === path);
    result('open_conversation', true, '');
    if (target) target.click();
    else location.assign(new URL(path, location.origin).href);
  }

  function openProject(path, result) {
    if (!PROJECT_PATH.test(path)) {
      return result('open_project', false, '项目地址无效。');
    }
    const target = projectPolicy && typeof projectPolicy.findNode === 'function'
      ? projectPolicy.findNode(document, path, isVisible, (node) => cleanText(
          node.getAttribute('data-project-title') ||
            node.getAttribute('title') ||
            node.getAttribute('aria-label') ||
            node.textContent
        ))
      : Array.from(document.querySelectorAll('a[href*="/g/g-p-"]'))
        .find((node) => sameOriginPath(node) === path);
    result('open_project', true, '');
    if (target) target.click();
    else location.assign(new URL(path, location.origin).href);
  }

  function capabilities() {
    const available = !!findSidebarButton(true) || conversationLinks().length > 0;
    return available ? ['conversation_list', 'conversation_search'] : [];
  }

  window.__elonChatGptConversations = Object.freeze({
    capabilities,
    newConversation,
    openConversation,
    openProject,
    requestList
  });
})();
