const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const source = fs.readFileSync(path.join(
  __dirname,
  '..',
  'android',
  'app',
  'src',
  'main',
  'assets',
  'chatgpt_web_adapter_snapshot_scheduler.js'
), 'utf8');
const context = { window: {} };
vm.runInNewContext(source, context);

function harness(options = {}) {
  let nextId = 1;
  let snapshots = 0;
  const timers = new Map();
  const scheduler = context.window.__elonChatGptSnapshotScheduler.create({
    scheduleTimer(delay, action) {
      const id = nextId++;
      timers.set(id, { delay, action });
      return id;
    },
    cancelTimer(id) {
      timers.delete(id);
    },
    snapshot() {
      snapshots += 1;
    },
    ...options
  });
  return {
    scheduler,
    timers,
    snapshots: () => snapshots,
    fireByDelay(delay) {
      const entry = Array.from(timers.entries()).find(([, timer]) => timer.delay === delay);
      assert.ok(entry, `missing ${delay}ms timer`);
      timers.delete(entry[0]);
      entry[1].action();
    }
  };
}

{
  const run = harness({
    quietDelayMs: 240,
    maxDelayMs: 5000,
    activeQuietDelayMs: 80,
    activeMaxDelayMs: 700
  });
  run.scheduler.schedule(false);
  assert.deepStrictEqual(
    Array.from(run.timers.values()).map((timer) => timer.delay).sort((a, b) => a - b),
    [240, 5000]
  );
  run.scheduler.schedule(true);
  assert.deepStrictEqual(
    Array.from(run.timers.values()).map((timer) => timer.delay).sort((a, b) => a - b),
    [80, 700]
  );
  run.fireByDelay(80);
  assert.strictEqual(run.snapshots(), 1);
  assert.strictEqual(run.timers.size, 0);
}

{
  const run = harness({
    quietDelayMs: 240,
    maxDelayMs: 5000,
    activeQuietDelayMs: 80,
    activeMaxDelayMs: 700
  });
  run.scheduler.schedule(true);
  run.scheduler.schedule(false);
  assert.deepStrictEqual(
    Array.from(run.timers.values()).map((timer) => timer.delay).sort((a, b) => a - b),
    [240, 700]
  );
  run.fireByDelay(700);
  assert.strictEqual(run.snapshots(), 1);
}

const adapterSource = fs.readFileSync(path.join(
  __dirname,
  '..',
  'android',
  'app',
  'src',
  'main',
  'assets',
  'chatgpt_web_adapter.js'
), 'utf8');
assert.match(adapterSource, /function isActiveMutation\(records\)/);
assert.match(adapterSource, /snapshotScheduler\.schedule\(active\)/);
assert.match(adapterSource, /quietDelayMs:\s*240/);
assert.match(adapterSource, /maxDelayMs:\s*5000/);
assert.match(adapterSource, /activeQuietDelayMs:\s*80/);
assert.match(adapterSource, /activeMaxDelayMs:\s*700/);

{
  const run = harness();
  run.scheduler.schedule();
  run.scheduler.schedule();
  run.scheduler.schedule();
  assert.deepStrictEqual(
    Array.from(run.timers.values()).map((timer) => timer.delay).sort((a, b) => a - b),
    [120, 1000]
  );
  run.fireByDelay(1000);
  assert.strictEqual(run.snapshots(), 1);
  assert.strictEqual(run.timers.size, 0);
}

{
  const run = harness();
  run.scheduler.schedule();
  run.fireByDelay(120);
  assert.strictEqual(run.snapshots(), 1);
  assert.strictEqual(run.timers.size, 0);
  run.scheduler.schedule();
  run.scheduler.dispose();
  assert.strictEqual(run.timers.size, 0);
  assert.strictEqual(run.snapshots(), 1);
}

console.log('CHATGPT_WEB_SNAPSHOT_SCHEDULER_TESTS=passed');
