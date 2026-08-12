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

function harness() {
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
    }
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
