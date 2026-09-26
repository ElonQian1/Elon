(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 4, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptRspackAttachments) {
    root.__elonChatGptRspackAttachments = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const runtime = page.__elonChatGptRspackRuntime;
  const context = page.__elonChatGptRspackContext?.create(page);
  let active = null, owned = null, code = 'idle';
  const entries = binding => binding.scope.get(binding.runtime.composer.f, binding.id);
  const same = (a, b) => Array.isArray(a) && a.length === b.length && a.every((entry, i) => entry === b[i]);
  function removeIds(job) {
    // Cancel only this upload's IDs, never reset the user's entire composer.
    for (const id of job.ids) job.binding.runtime.composer.E(job.binding.scope, job.binding.id, id, 'chat', 'native_cancel');
  }
  function cancel() {
    if (!active) return;
    active.cancelled = true;
    removeIds(active);
    active.abort?.();
  }
  async function upload(files, descriptor, signal) {
    if (active) throw Error('attachment_busy');
    const loaded = await runtime?.load();
    if (!['web_20260926_rspack', 'web_20260926b_rspack'].includes(runtime?.profile) || typeof loaded?.composer.N !== 'function' ||
        typeof loaded?.composer.E !== 'function' || typeof loaded?.attachments.m !== 'function') throw Error('attachment_runtime_pending');
    const binding = context.find();
    if (!binding || binding.href !== descriptor.href || binding.token !== descriptor.documentToken ||
        !Array.isArray(files) || !files.length || files.length > 9 || signal?.aborted) throw Error('attachment_context_changed');
    const job = { binding, ids: [], cancelled: false, abort: null };
    active = job;
    let timer, guard;
    const current = () => !job.cancelled && !signal?.aborted && context.owns(binding);
    signal?.addEventListener('abort', cancel, { once: true });
    try {
      code = 'uploading';
      const interrupted = new Promise((_, reject) => {
        job.abort = () => reject(Error('attachment_cancelled'));
        timer = page.setTimeout(() => { code = 'upload_timeout'; cancel(); }, options.timeoutMs || 90000);
        guard = page.setInterval(() => { if (!current()) cancel(); }, 500);
      });
      // vG.N is the reviewed official upload transaction: quota, conversion,
      // reservation, bytes, processing and scoped ready entries stay together.
      const request = Promise.resolve().then(() => {
        if (!current() || !context.current(binding)) throw Error('attachment_context_changed');
        return loaded.composer.N(binding.scope, binding.id, files, {
          isTemporaryChat: binding.temporary,
          onQueued(ids) {
            job.ids = Array.isArray(ids) ? [...ids] : [];
            if (!current() || job.ids.length !== files.length) { job.cancelled = true; removeIds(job); }
          },
        });
      }).finally(() => {
        if (job.cancelled) removeIds(job);
        if (active === job) active = null;
      });
      await Promise.race([request, interrupted]);
      const ready = entries(binding);
      if (!current() || job.ids.length !== files.length || new Set(job.ids).size !== files.length ||
          !Array.isArray(ready) || ready.length !== files.length ||
          !ready.every(entry => job.ids.includes(entry.uploadId) && entry.status === 'ready' &&
            typeof entry.id === 'string' && entry.id.length > 0) || !context.capture(binding.node, ready)) {
        throw Error('attachment_association_unconfirmed');
      }
      owned = { binding, ready: [...ready] };
      code = 'associated';
    } catch (error) {
      job.cancelled = true; removeIds(job);
      if (code !== 'upload_timeout') code = 'upload_unconfirmed';
      throw error;
    } finally {
      page.clearTimeout(timer); page.clearInterval(guard);
      signal?.removeEventListener('abort', cancel);
      // A timed-out official operation keeps single-flight until it settles.
    }
  }
  function prepare(node) {
    const value = owned;
    const fail = reason => { code = reason; return null; };
    if (!value) return fail('attachment_not_owned');
    if (!same(entries(value.binding), value.ready)) return fail('attachment_entries_changed');
    const owner = context.read(node);
    if (!owner) return fail(context.state().code);
    // React may replace the editor after an upload. The committed account,
    // conversation and exact file entries own the lease, not the old DOM node.
    if (!['id', 'token', 'href', 'accountId', 'userId', 'generation'].every(key => owner[key] === value.binding[key]) ||
        owner.scope.node !== value.binding.scope.node) return fail('attachment_owner_changed');
    const binding = context.capture(node, value.ready);
    if (!binding) return fail(context.state().code === 'attachments_or_tools_present'
      ? 'attachment_tools_changed' : context.state().code);
    if (binding.model.slug !== value.binding.model.slug) {
      // Model hydration may finish after upload. Use the reviewed website's
      // routing check, not slug equality, to validate these exact ready files.
      try {
        if (runtime.profile !== 'web_20260926b_rspack' || typeof binding.runtime.composer.x !== 'function' ||
            binding.runtime.composer.x(binding.scope, binding.id, binding.model,
              binding.scope.get(binding.runtime.conversation.w, binding.id), value.ready) !== true) {
          return fail('attachment_model_changed');
        }
      } catch (_) { return fail('attachment_model_changed'); }
    }
    const attachments = binding.runtime.attachments.m(value.ready);
    if (attachments.length !== value.ready.length) return fail('attachment_projection_changed');
    const fingerprint = JSON.stringify(attachments);
    let consumed = false;
    const current = () => !consumed && owned === value && context.owns(binding) &&
      same(entries(binding), value.ready) && JSON.stringify(binding.runtime.attachments.m(value.ready)) === fingerprint;
    code = 'associated';
    return { binding, attachments: JSON.parse(fingerprint), current,
      consumeAccepted() {
        if (consumed) return;
        // ACK can remount the editor. Remove only the captured ready entries.
        binding.scope.set(binding.runtime.composer.f, binding.id, existing => existing.filter(item => !value.ready.includes(item)));
        consumed = true;
        if (owned === value) owned = null;
      } };
  }
  function merge(dom) {
    if (!owned || !context.owns(owned.binding) || !same(entries(owned.binding), owned.ready)) return dom;
    return owned.ready.map(item => ({ id: 'private_attachment_' + item.uploadId, name: item.name, state: 'ready', removable: true }));
  }
  function remove(id) {
    if (!owned || !context.owns(owned.binding)) return false;
    const entry = owned.ready.find(item => id === 'private_attachment_' + item.uploadId);
    if (!entry || !entries(owned.binding).includes(entry)) return false;
    owned.binding.runtime.composer.E(owned.binding.scope, owned.binding.id, entry.uploadId, 'chat');
    if (entries(owned.binding).includes(entry)) return false;
    owned.ready = owned.ready.filter(item => item !== entry);
    if (!owned.ready.length) owned = null;
    return true;
  }
  return Object.freeze({ version: 4, upload, prepare, cancel, merge, remove,
    state: () => ({ code, pending: !!active, count: owned?.ready.length || 0 }) });
});
