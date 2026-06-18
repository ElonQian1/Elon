(function () {
  const PLATFORM_ORDER = ['android', 'windows', 'web', 'ios', 'macos', 'linux'];
  const PLATFORM_META = {
    android: { label: 'Android APK', short: 'APK' },
    windows: { label: 'Windows 客户端', short: 'Win' },
    web: { label: '网页端', short: 'Web' },
    ios: { label: 'iOS', short: 'iOS' },
    macos: { label: 'macOS', short: 'Mac' },
    linux: { label: 'Linux', short: 'Linux' }
  };
  const ACTIVE_STATUSES = new Set(['available', 'external']);

  function create(ctx) {
    const {
      state, els, clean, escapeHtml, firstChar, formatTime, titleOf, iconUrlOf,
      channelName, channelGlyph, selectProjectChannel, setHeader, setComposer, setNodeMode
    } = ctx;

    function projectOf() {
      const spaceProject = state.projectSpace && state.projectSpace.project;
      if (spaceProject) return spaceProject;
      return (state.projects || []).find((item) => String(item.id) === String(state.activeProjectId)) || null;
    }

    function landingOf(project) {
      return objectOf(
        (state.projectSpace && (state.projectSpace.landing || state.projectSpace.project_landing || state.projectSpace.projectLanding)),
        project && (project.landing || project.project_landing || project.projectLanding)
      );
    }

    function objectOf() {
      for (const value of arguments) {
        if (value && typeof value === 'object' && !Array.isArray(value)) return value;
      }
      return {};
    }

    function arrayOf() {
      for (const value of arguments) {
        if (Array.isArray(value)) return value;
      }
      return [];
    }

    function downloadArrayOf(value) {
      if (Array.isArray(value)) return value;
      if (!value || typeof value !== 'object') return [];
      return Object.entries(value).map(([platform, item]) => {
        if (item && typeof item === 'object' && !Array.isArray(item)) {
          return Object.assign({ platform }, item);
        }
        return { platform, url: item };
      });
    }

    function valueOf() {
      for (const value of arguments) {
        const next = clean(value);
        if (next) return next;
      }
      return '';
    }

    function formatBytes(value) {
      const size = Number(value);
      if (!Number.isFinite(size) || size <= 0) return '';
      if (size >= 1048576) {
        const mb = size / 1048576;
        return '约 ' + (mb >= 10 ? mb.toFixed(0) : mb.toFixed(1)) + ' MB';
      }
      return '约 ' + Math.max(1, Math.round(size / 1024)) + ' KB';
    }
    function platformOf(value) {
      const raw = clean(value).toLowerCase().replace(/[_\s-]+/g, '');
      if (raw === 'win' || raw === 'window' || raw === 'windowsclient') return 'windows';
      if (raw === 'apk' || raw === 'androidapk') return 'android';
      if (raw === 'mac' || raw === 'osx' || raw === 'darwin') return 'macos';
      if (raw === 'browser' || raw === 'h5' || raw === 'website') return 'web';
      return PLATFORM_META[raw] ? raw : '';
    }

    function statusOf(item) {
      const explicit = clean(item.status || item.availability || item.health_status || item.healthStatus)
        .toLowerCase();
      if (['available', 'external', 'unavailable', 'coming_soon', 'needs_configuration', 'third_party', 'planned', 'pending'].includes(explicit)) {
        return explicit;
      }
      if (valueOf(item.url, item.download_url, item.downloadUrl, item.href)) return 'available';
      if (valueOf(item.manifest_url, item.manifestUrl)) return 'pending';
      return 'planned';
    }

    function statusLabel(status) {
      if (status === 'available') return '可下载';
      if (status === 'external') return '外部入口';
      if (status === 'unavailable') return '暂不可用';
      if (status === 'coming_soon') return '即将支持';
      if (status === 'needs_configuration') return '待配置';
      if (status === 'third_party') return '第三方方案';
      if (status === 'pending') return '待检查';
      if (status === 'planned') return '计划中';
      return '待配置';
    }

    function normalizeDownload(item) {
      const platform = platformOf(item.platform || item.os || item.kind || item.type);
      if (!platform) return null;
      const meta = PLATFORM_META[platform];
      const status = statusOf(item);
      return {
        platform,
        label: valueOf(item.label, item.name, meta.label),
        short: valueOf(item.short, item.badge, meta.short),
        url: valueOf(item.url, item.download_url, item.downloadUrl, item.fallback_url, item.fallbackUrl, item.href),
        manifestUrl: valueOf(item.manifest_url, item.manifestUrl),
        version: valueOf(item.version_name, item.versionName, item.version, item.build),
        size: valueOf(item.size_label, item.sizeLabel, item.size, formatBytes(item.size_bytes || item.sizeBytes || item.file_size || item.fileSize)),
        status,
        note: valueOf(item.note, item.description, item.health_error, item.healthError, item.changelog, item.release_notes, item.releaseNotes),
        external: !!item.external || status === 'external'
      };
    }

    function downloadUrlWithToken(url) {
      const raw = clean(url);
      if (!raw) return '';
      try {
        const target = new URL(raw, window.location.origin);
        if (target.origin === window.location.origin && state.token && !target.searchParams.has('token')) {
          target.searchParams.set('token', state.token);
        }
        return target.toString();
      } catch (_) {
        return raw;
      }
    }

    function latestApkUrl(project) {
      return valueOf(
        state.projectSpace && (state.projectSpace.latest_apk_url || state.projectSpace.latestApkUrl),
        project && (project.latest_apk_url || project.latestApkUrl || project.last_apk_url || project.lastApkUrl)
      );
    }

    function webUrlOf(project, landing) {
      return valueOf(
        landing.web_url, landing.webUrl, landing.website_url, landing.websiteUrl,
        project && (project.web_url || project.webUrl || project.website_url || project.websiteUrl)
      );
    }

    function customLandingUrlOf(project, landing) {
      return valueOf(
        landing.custom_landing_url, landing.customLandingUrl,
        project && (project.custom_landing_url || project.customLandingUrl || project.promote_url || project.promoteUrl)
      );
    }

    function downloadsOf(project, landing) {
      const byPlatform = new Map();
      const rawDownloads = [
        ...downloadArrayOf(state.projectSpace && state.projectSpace.downloads),
        ...downloadArrayOf(landing.downloads),
        ...downloadArrayOf(project && project.downloads)
      ];
      rawDownloads.map(normalizeDownload).filter(Boolean).forEach((item) => byPlatform.set(item.platform, item));

      const apkUrl = latestApkUrl(project);
      if (apkUrl && !byPlatform.has('android')) {
        byPlatform.set('android', {
          platform: 'android',
          label: PLATFORM_META.android.label,
          short: PLATFORM_META.android.short,
          url: apkUrl,
          manifestUrl: '',
          version: '',
          size: '',
          status: 'available',
          note: '项目最新可安装包'
        });
      }

      const webUrl = webUrlOf(project, landing);
      if (webUrl && !byPlatform.has('web')) {
        byPlatform.set('web', {
          platform: 'web',
          label: PLATFORM_META.web.label,
          short: PLATFORM_META.web.short,
          url: webUrl,
          manifestUrl: '',
          version: '',
          size: '',
          status: 'available',
          note: '浏览器访问入口'
        });
      }

      return PLATFORM_ORDER.map((platform) => byPlatform.get(platform) || {
        platform,
        label: PLATFORM_META[platform].label,
        short: PLATFORM_META[platform].short,
        url: '',
        manifestUrl: '',
        version: '',
        size: '',
        status: 'planned',
        note: platform === 'android' ? '暂无 APK' : '等待项目配置'
      });
    }

    function descriptionOf(project, landing) {
      return valueOf(
        landing.summary, landing.description, landing.overview,
        project && (project.project_description || project.projectDescription || project.description || project.subtitle),
        '这个项目由一龙平台托管，已接入项目协作、AI 开发和交付下载流程。'
      );
    }

    function taglineOf(project, landing, downloads) {
      return valueOf(
        landing.tagline, landing.slogan, project && (project.tagline || project.slogan),
        downloads.some((item) => item.status === 'available') ? '项目介绍与多端下载入口' : '项目首页正在完善中'
      );
    }

    function featuresOf(project, landing) {
      const raw = arrayOf(landing.highlights, landing.features, project && project.highlights, project && project.features);
      const values = raw.map((item) => typeof item === 'string' ? clean(item) : valueOf(item.title, item.text, item.label))
        .filter(Boolean)
        .slice(0, 4);
      if (values.length) return values;
      return ['统一展示项目简介', '集中管理下载入口', '保留公告、文档和 AI 开发频道'];
    }

    function targetUsersOf(project, landing) {
      const raw = arrayOf(landing.target_users, landing.targetUsers, project && project.target_users, project && project.targetUsers);
      return raw.map((item) => typeof item === 'string' ? clean(item) : valueOf(item.title, item.text, item.label))
        .filter(Boolean)
        .slice(0, 3);
    }

    function channelByKind(kind) {
      return ((state.projectSpace && state.projectSpace.channels) || [])
        .find((channel) => clean(channel.kind || channel.channel_kind).toLowerCase() === kind);
    }

    function quickChannels() {
      return ['announcements', 'docs', 'ai_development', 'builds', 'issues']
        .map((kind) => channelByKind(kind))
        .filter(Boolean);
    }

    function iconHtml(project) {
      const icon = iconUrlOf(project);
      const name = titleOf(project);
      if (icon) {
        return `<span class="project-landing-icon"><img src="${escapeHtml(icon)}" alt="" onerror="this.remove(); this.parentElement.classList.add('fallback')" /><span>${escapeHtml(firstChar(name, '项'))}</span></span>`;
      }
      return `<span class="project-landing-icon fallback"><span>${escapeHtml(firstChar(name, '项'))}</span></span>`;
    }

    function downloadCardHtml(item) {
      const enabled = ACTIVE_STATUSES.has(item.status) && item.url;
      const detail = [item.version, item.size, item.note].filter(Boolean).join(' · ');
      return `<button class="project-landing-download ${enabled ? '' : 'disabled'} status-${escapeHtml(item.status)}" type="button"
          data-download-url="${escapeHtml(enabled ? item.url : '')}" aria-disabled="${enabled ? 'false' : 'true'}">
        <span class="project-landing-platform">${escapeHtml(item.short)}</span>
        <span class="project-landing-download-main">
          <strong>${escapeHtml(item.label)}</strong>
          <span>${escapeHtml(detail || statusLabel(item.status))}</span>
        </span>
        <em>${escapeHtml(statusLabel(item.status))}</em>
      </button>`;
    }

    function resourceButtonsHtml(project, landing) {
      const customLandingUrl = customLandingUrlOf(project, landing);
      const resources = [];
      if (customLandingUrl) resources.push({ label: '完整介绍', url: customLandingUrl });
      const manifestUrl = valueOf(landing.landing_manifest_url, landing.landingManifestUrl, project && (project.landing_manifest_url || project.landingManifestUrl));
      if (manifestUrl) resources.push({ label: 'Landing Manifest', url: manifestUrl });
      [
        ...arrayOf(landing.resources),
        ...arrayOf(project && project.resources)
      ].forEach((item) => {
        const url = valueOf(item && (item.url || item.href || item.link));
        const label = valueOf(item && (item.label || item.name || item.title), '相关链接');
        if (url) resources.push({ label, url });
      });
      return resources.map((resource) =>
        `<button class="project-landing-resource" type="button" data-resource-url="${escapeHtml(resource.url)}">${escapeHtml(resource.label)}</button>`
      ).join('');
    }

    function quickChannelHtml(channel) {
      return `<button class="project-landing-channel" type="button" data-channel-id="${escapeHtml(channel.id)}">
        <span>${escapeHtml(channelGlyph(channel))}</span>
        <strong>${escapeHtml(channelName(channel))}</strong>
      </button>`;
    }

    function updateLine(project) {
      const time = valueOf(project && (project.updated_at || project.updatedAt), state.projectSpace && state.projectSpace.updated_at);
      return time ? `最近更新 ${formatTime(time)}` : '项目资料待更新';
    }

    function render() {
      const project = projectOf();
      if (!project) return;
      const landing = landingOf(project);
      const downloads = downloadsOf(project, landing);
      const title = titleOf(project);
      const tagline = taglineOf(project, landing, downloads);
      const description = descriptionOf(project, landing);
      const features = featuresOf(project, landing);
      const targets = targetUsersOf(project, landing);
      const channels = quickChannels();
      const resourceButtons = resourceButtonsHtml(project, landing);

      setNodeMode(false);
      setHeader('首', title, '项目介绍与下载');
      setComposer(false, '从左侧选择频道后开始输入', false);
      els.messageList.classList.add('project-landing-mode');
      els.messageList.innerHTML = `<section class="project-landing">
        <header class="project-landing-hero">
          ${iconHtml(project)}
          <div class="project-landing-hero-copy">
            <div class="project-landing-kicker">${escapeHtml(updateLine(project))}</div>
            <h1>${escapeHtml(title)}</h1>
            <p>${escapeHtml(tagline)}</p>
          </div>
        </header>
        <div class="project-landing-summary">${escapeHtml(description)}</div>
        <div class="project-landing-downloads">${downloads.map(downloadCardHtml).join('')}</div>
        <div class="project-landing-section">
          <h2>核心信息</h2>
          <div class="project-landing-feature-grid">${features.map((item) => `<span>${escapeHtml(item)}</span>`).join('')}</div>
        </div>
        ${targets.length ? `<div class="project-landing-section"><h2>适用人群</h2><div class="project-landing-tag-list">${targets.map((item) => `<span>${escapeHtml(item)}</span>`).join('')}</div></div>` : ''}
        <div class="project-landing-footer">
          <div class="project-landing-channels">${channels.map(quickChannelHtml).join('')}</div>
          <div class="project-landing-resources">${resourceButtons}</div>
        </div>
      </section>`;

      bindActions();
    }

    function bindActions() {
      els.messageList.querySelectorAll('[data-download-url]').forEach((button) => {
        button.addEventListener('click', () => {
          const url = downloadUrlWithToken(button.dataset.downloadUrl);
          if (url) window.open(url, '_blank', 'noreferrer');
        });
      });
      els.messageList.querySelectorAll('.project-landing-channel[data-channel-id]').forEach((button) => {
        button.addEventListener('click', () => selectProjectChannel(button.dataset.channelId));
      });
      els.messageList.querySelectorAll('[data-resource-url]').forEach((button) => {
        button.addEventListener('click', () => {
          const url = clean(button.dataset.resourceUrl);
          if (url) window.open(url, '_blank', 'noreferrer');
        });
      });
    }

    return { render };
  }

  window.ElonPcProjectLanding = { create };
})();
