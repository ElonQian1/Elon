'use strict';
const assert = require('node:assert/strict');
const path = require('node:path');
const assets = path.join(__dirname, '../../android/app/src/main/assets');
const contractModule = require(path.join(assets, 'chatgpt_web_private_model_contract.js'));
const runtimeModule = require(path.join(assets, 'chatgpt_web_private_model_state.js'));
const CID = '00000000-0000-0000-0000-000000000001';

function fixture(options = {}) {
  const writes = [], events = [], results = [], timers = new Set();
  let model = 'fixture-fast', effort = 'low', tier, serviceTier, version = 'fixture-version';
  let account = 'Bearer fixture-not-a-credential', loaded = true, work = false;
  const conversation = { serverId$: () => CID };
  const fast = { id: 'fixture-fast' }, thinking = { id: 'fixture-thinking' };
  const menu = { conversation, modelSwitcherDenialsBySlug: {},
    modelsData: { models: new Map([[fast.id, fast], [thinking.id, thinking]]) },
    composerIntelligencePickerState: {
      selectedVersionEntry: { id: 'fixture-version' },
      bucketSelections: [
        { bucket: 0, title: 'Low', availability: { status: 'available' }, modelSlug: fast.id,
          modelConfig: fast, thinkingEffort: 'low', category: { modelLane: 'instant' } },
        { bucket: 1, title: 'High', availability: { status: 'available' }, modelSlug: thinking.id,
          modelConfig: thinking, thinkingEffort: 'high', category: { modelLane: 'thinking' } },
        { bucket: 2, title: 'Extra High', availability: { status: 'available' }, modelSlug: thinking.id,
          modelConfig: thinking, thinkingEffort: 'max', category: { modelLane: 'pro' } }
      ]
    } };
  const props = { ariaDisabled: false, dropdownOpen: false, dropdownContent: { props: menu } };
  const top = { stateNode: {} }; top.stateNode.current = top;
  const ancestor = { memoizedProps: props, return: top }, host = { return: ancestor };
  top.child = ancestor; ancestor.child = host;
  const node = { isConnected: true, __reactFiber$fixture: host,
    getAttribute: name => name === 'aria-expanded' ? String(props.dropdownOpen) : null,
    getBoundingClientRect: () => ({ width: 80, height: 40, left: 0, top: 0 }) };
  const store = { conversationThinkingEffort$: () => effort,
    setThinkingEffort(value, slug) { writes.push(['effort', value, slug]); effort = value; } };
  const tierStore = { getDraftServiceTier: () => tier, conversationServiceTier$: () => serviceTier,
    setServiceTier(value) { writes.push(['tier', value]); tier = serviceTier = value; } };
  const baseSelections = menu.composerIntelligencePickerState.bucketSelections;
  const nextSelections = baseSelections.slice(0, 2).map(item => ({ ...item, modelSlug: item.modelSlug + '-v2',
    modelConfig: { id: item.modelSlug + '-v2' }, thinkingEffortLane: item.category.modelLane }));
  function syncPicker() {
    if (!options.extended) return;
    const state = menu.composerIntelligencePickerState;
    state.selectedVersionEntry = menu.modelsData.versions.find(item => item.id === version);
    state.bucketSelections = version === 'fixture-version' ? baseSelections : nextSelections;
    state.currentSelection = state.bucketSelections.find(item => item.modelSlug === model && item.thinkingEffort === effort);
    state.currentBucket = state.currentSelection?.bucket;
  }
  const modules = {
    shared: { M$: fn => fn(), uo: () => work, t4: { PRO: 'pro' },
      RW: () => ({ mutate: data => writes.push(['preference', data]) }) },
    conversation: {
      Nrn: () => ({ id: model }), yRt: () => store, l0: () => tierStore,
      Grn(conv, value) { assert.equal(conv, conversation); writes.push(['model', value]); model = value; syncPicker(); },
      vRt: () => ['low', 'high', 'max'],
      p8t: ({ modelSlug, modelSwitcherDenialsBySlug: denials }) => ({ status: denials[modelSlug] ? 'unavailable' : 'available' }),
      M1t: () => ({ mutate: data => writes.push(['pro-preference', data]) }),
      Rdn: { CHATGPT_MODEL_PICKER_SURFACE_COMPOSER: 'CHATGPT_MODEL_PICKER_SURFACE_COMPOSER' }
    },
    composer: { Ih(command) {
      writes.push(['official-model-action', command.modelId, command.thinkingEffort]);
      command.applyModelSelection(command.conversation, command.modelId);
    } }
  };
  if (options.extended) {
    nextSelections.forEach(item => menu.modelsData.models.set(item.modelSlug, item.modelConfig));
    for (const item of [...baseSelections, ...nextSelections]) {
      item.serviceTierOptions = [{ service_tier: 'standard' }, { service_tier: 'fast' }];
      item.defaultServiceTier = 'standard';
    }
    menu.modelsData.versions = [
      { id: 'fixture-version', displayText: 'Version 1', slugs: [fast.id, thinking.id], intelligencePresets: baseSelections },
      { id: 'fixture-version-v2', displayText: 'Version 2', slugs: nextSelections.map(item => item.modelSlug), intelligencePresets: nextSelections }
    ];
    menu.modelsData.categories = [...baseSelections.slice(0, 2), ...nextSelections].map(item =>
      ({ ...item.category, defaultModel: item.modelSlug, supportedModels: [item.modelSlug] }));
    modules.shared.t4 = { PRO: 'pro', AUTO: 'auto', INSTANT: 'instant' };
    modules.shared.IX = (data, slug) => data.categories.find(item => item.supportedModels.includes(slug));
    Object.assign(modules.conversation, {
      Rrn: () => ({ conversationVersion$: () => version,
        setConversationVersion(value) { writes.push(['version', value]); version = value; } }),
      win: (data, version) => data.categories.filter(item => version.slugs.includes(item.defaultModel)),
      Ein: value => value.displayText, ay: value => value.intelligencePresets,
      iy: ({ autoSwitcherEnabled, getModelAvailability, intelligencePresets, selectedVersionEntry }) => {
        assert.equal(typeof autoSwitcherEnabled, 'boolean');
        assert.equal(menu.modelsData.versions.find(item => item.id === selectedVersionEntry.id), selectedVersionEntry);
        return intelligencePresets.map(item => ({ ...item, availability: getModelAvailability(item.modelSlug) }));
      },
      ry: ({ bucketSelections, currentBucket, currentLane }) => {
        const available = bucketSelections.filter(item => item.availability.status === 'available');
        return available.find(item => item.bucket === currentBucket && item.category.modelLane === currentLane) ??
          available.find(item => item.category.modelLane === currentLane) ?? available[0];
      },
      Jrn: () => ({ juices: { pro: 'max' } }), Hrn: () => true,
      f8t: input => modules.conversation.p8t(input).status === 'available',
      c0: ({ configuredServiceTier, defaultServiceTier, serviceTierOptions }) =>
        serviceTierOptions.some(item => item.service_tier === configuredServiceTier) ? configuredServiceTier :
          serviceTierOptions.some(item => item.service_tier === defaultServiceTier) ? defaultServiceTier : serviceTierOptions[0]?.service_tier,
    });
    syncPicker();
  }
  const page = { location: { href: 'https://chatgpt.com/c/' + CID },
    __elonChatGptPrivateModelCatalog: options.modelCatalog,
    __elonChatGptDocumentToken: 'doc_fixture_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    document: { querySelector: () => null }, performance: { getEntriesByName: () => loaded ? [{}] : [] },
    setTimeout(fn) { timers.add(fn); return fn; }, clearTimeout(fn) { timers.delete(fn); } };
  const contract = contractModule.create(page), getTrigger = () => node;
  let imports = 0, fallback = 0, snapshots = 0, advanced = 0;
  const runtime = runtimeModule.create(page, { contract: contractModule, loadRuntime: async url => {
    imports++;
    const key = Object.keys(contract.urls).find(key => contract.urls[key] === url);
    assert.ok(key, 'no guessed runtime URL');
    return options.loadRuntime ? options.loadRuntime(key, modules[key]) : modules[key];
  } });
  const result = (...row) => results.push(row), emit = value => events.push(value);
  const request = () => runtime.request(getTrigger, emit, result, () => { fallback++; });
  const select = id => runtime.select(id, result, () => { snapshots++; }, () => { advanced++; });
  return { page, node, host, top, ancestor, props, menu, modules, writes, events, results, timers,
    conversation, contract, runtime, getTrigger, request, select, syncPicker, tierStore,
    choice: label => events.at(-1).find(item => item.label === label),
    get model() { return model; }, set model(v) { model = v; syncPicker(); },
    get effort() { return effort; }, set effort(v) { effort = v; syncPicker(); },
    get version() { return version; }, set version(v) { version = v; syncPicker(); },
    get serviceTier() { return serviceTier; }, set serviceTier(v) { serviceTier = v; },
    set tier(v) { tier = v; }, set account(v) { account = v; }, set loaded(v) { loaded = v; }, set work(v) { work = v; },
    get imports() { return imports; }, get fallback() { return fallback; },
    get advanced() { return advanced; }, get snapshots() { return snapshots; } };
}

module.exports = { fixture };
