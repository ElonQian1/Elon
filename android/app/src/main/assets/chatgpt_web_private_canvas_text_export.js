(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasTextExport = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  // Official Z table in bc86e6a9-lyyfjzfq9uy5wnj2.js; unknown languages are not guessed.
  const sourceTypes = Object.freeze(Object.fromEntries(Object.entries({
    bash: ['sh', 'text/x-shellscript'], zsh: ['zsh', 'text/x-shellscript'],
    javascript: ['js', 'application/javascript'], typescript: ['ts', 'application/typescript'],
    html: ['html', 'text/html'], css: ['css', 'text/css'], python: ['py', 'text/x-python'],
    json: ['json', 'application/json'], sql: ['sql', 'application/sql'], go: ['go', 'text/x-go'],
    yaml: ['yaml', 'application/x-yaml'], java: ['java', 'text/x-java-source'], rust: ['rs', 'text/rust'],
    cpp: ['cpp', 'text/x-c++src'], swift: ['swift', 'text/swift'], php: ['php', 'application/x-httpd-php'],
    xml: ['xml', 'application/xml'], ruby: ['rb', 'text/x-ruby'], haskell: ['hs', 'text/x-haskell'],
    kotlin: ['kt', 'text/x-kotlin'], csharp: ['cs', 'text/x-csharp'], c: ['c', 'text/x-csrc'],
    objectivec: ['m', 'text/x-objectivec'], r: ['r', 'text/plain'], lua: ['lua', 'text/x-lua'],
    dart: ['dart', 'text/x-dart'], scala: ['scala', 'text/x-scala'], perl: ['pl', 'text/x-perl'],
    commonlisp: ['lisp', 'text/x-common-lisp'], clojure: ['clj', 'text/x-clojure'], ocaml: ['ml', 'text/x-ocaml'],
    powershell: ['ps1', 'text/x-powershell'], verilog: ['v', 'text/x-verilog'],
    dockerfile: ['Dockerfile', 'text/x-dockerfile'], vue: ['vue', 'text/x-vue'],
    react: ['jsx', 'application/javascript'], other: ['txt', 'text/plain']
  }).map(([type, [extension, mediaType]]) => ['code/' + type, Object.freeze({ extension, mediaType })])));
  const documents = Object.freeze({ md: Object.freeze({ extension: 'md', mediaType: 'text/markdown' }),
    pdf: Object.freeze({ extension: 'pdf', mediaType: 'application/pdf' }),
    docx: Object.freeze({ extension: 'docx', mediaType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }) });
  const PROFILE = 'web_20260912', MARKERS = Object.freeze(['contentReference', 'hiveTranscript']);
  const MODULES = Object.freeze(['conversation-small-h1dtzoris1y9588z.js', 'e5d54aa7-o5mtxxnk4j8zox9y.js',
    '1c4de3ec-ix5n1yyu8whxeib8.js', '6afb0137-dqge0sx8jli56ai8.js']);
  const fail = code => { throw Error('download_' + code); };

  function describe(type, format) {
    if (type === 'document' && Object.hasOwn(documents, format)) return documents[format];
    return format === 'source' && Object.hasOwn(sourceTypes, type) ? sourceTypes[type] : null;
  }

  function validFile(file) {
    const format = /^canvas-export-[A-Za-z0-9_-]{1,128}-(pdf|docx|md|source)$/.exec(file?.id || '')?.[1];
    if (!format || typeof file.name !== 'string' || !file.name.trim() || file.name.length > 200 ||
        /[\x00-\x1f\x7f-\x9f\u202a-\u202e\u2066-\u2069\\/:*?"<>|]/.test(file.name)) return false;
    const choices = format === 'source' ? Object.values(sourceTypes) : [documents[format]];
    return choices.some(row => row.mediaType === file.mediaType && file.name.endsWith('.' + row.extension));
  }

  function create(page, options = {}) {
    let loaded;
    const profile = () => page.__elonChatGptPrivateRuntimeBindings?.state?.().profile_id;
    const load = options.loadRuntime || (url => import(url));
    function bounded(work) {
      let timer;
      return Promise.race([work,
        new Promise((_, reject) => { timer = page.setTimeout(() => reject(Error('download_transfer_timeout')), options.timeoutMs || 1500); })
      ]).finally(() => page.clearTimeout(timer));
    }
    function plugins() {
      if (loaded) return loaded;
      const attempt = bounded(Promise.all(MODULES.map(file =>
        Promise.resolve().then(() => load('https://chatgpt.com/cdn/assets/' + file))))).then(([unified, remark, strip, hive]) => {
        if (profile() !== PROFILE || typeof unified.IAn !== 'function' || typeof remark.n !== 'function' ||
            typeof strip.t !== 'function' || typeof hive.i !== 'function') fail('source_unsupported');
        unified.IAn(); remark.n(); strip.t(); hive.i();
        const value = { unified: unified.LAn?.unified, remark: remark.r?.CANVAS_REMARK_PLUGINS,
          strip: strip.r?.stripDirectivePlugin, hive: hive.r?.hiveLogDirectivePlugin };
        if (typeof value.unified !== 'function' || !Array.isArray(value.remark) ||
            typeof value.strip !== 'function' || typeof value.hive !== 'function') fail('source_unsupported');
        return value;
      });
      loaded = attempt;
      attempt.catch(() => { if (loaded === attempt) loaded = null; });
      return attempt;
    }
    async function serialize(document, format, check) {
      check();
      if (!describe(document.documentType, format) || !['md', 'source'].includes(format)) fail('source_unsupported');
      page.__elonChatGptPrivateCanvasDocumentPolicy.offsets(document.content);
      // The official G preserves plaintext verbatim unless a known directive requires its remark pipeline.
      if (format === 'source' || MARKERS.every(marker => !document.content.includes(marker))) return document.content;
      if (profile() !== PROFILE) fail('source_unsupported');
      const value = await plugins();
      check();
      if (profile() !== PROFILE) fail('source_unsupported');
      const pipeline = value.unified();
      pipeline.use(value.hive).use(value.strip, { preserve: undefined }).use(value.remark);
      const result = String(await bounded(Promise.resolve().then(() => pipeline.process(document.content)))).trim();
      check();
      if (profile() !== PROFILE) fail('source_unsupported');
      page.__elonChatGptPrivateCanvasDocumentPolicy.offsets(result);
      return result;
    }
    return Object.freeze({ serialize });
  }
  return Object.freeze({ version: 1, sourceTypes, describe, validFile, create });
});
