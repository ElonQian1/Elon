import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, clone, reply, environment, authorize } from './hardening-helper.js';

function page(start, end, count) {
  const value = clone(fixture.valid.asset_first);
  value.balance = { total_base_units: '100000', reserved_base_units: '0', available_base_units: '100000' };
  value.progress = {
    request_count: String(count), open_count: '0', range_start: String(start), range_end: String(end),
    requests: Array.from({ length: end - start + 1 }, (_, offset) => ({
      request_id: `eskpsr_${String(start + offset).padStart(32, '0')}`,
      amount_base_units: '1', status: 'canceled',
      created_at: '2029-12-30T12:00:00Z', canceled_at: '2029-12-30T13:00:00Z',
    })),
    has_more: end < count, next_cursor: end < count ? `cursor-${end}` : null,
  };
  return value;
}

test('nonadjacent pages cannot repeat a request ID even when ranges keep increasing', async () => {
  const env = environment();
  await authorize(env);
  const firstPage = page(1, 1, 3), secondPage = page(2, 2, 3), thirdPage = page(3, 3, 3);
  thirdPage.progress.requests[0].request_id = firstPage.progress.requests[0].request_id;
  env.queue.push(reply(firstPage), reply(secondPage), reply(thirdPage));
  const first = await env.client.readAssets({ limit: 1 });
  const second = await env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor });
  await assert.rejects(env.client.readAssets({ limit: 1, cursor: second.page.progress.next_cursor }),
    { code: 'invalid_response' });
  assert.equal(env.client.state.status, 'unauthenticated');
  assert.equal(env.client.state.has_snapshot, false);
});

test('a nonadjacent cursor cycle is rejected even when all request IDs are distinct', async () => {
  const env = environment();
  await authorize(env);
  const firstPage = page(1, 1, 4), secondPage = page(2, 2, 4), thirdPage = page(3, 3, 4);
  thirdPage.progress.next_cursor = firstPage.progress.next_cursor;
  env.queue.push(reply(firstPage), reply(secondPage), reply(thirdPage));
  const first = await env.client.readAssets({ limit: 1 });
  const second = await env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor });
  await assert.rejects(env.client.readAssets({ limit: 1, cursor: second.page.progress.next_cursor }),
    { code: 'invalid_response' });
  assert.equal(env.client.state.has_snapshot, false);
  assert.equal(env.client.state.status, 'unauthenticated');
});

test('an explicit first-page refresh starts a new chain and permits prior IDs and cursors', async () => {
  const env = environment();
  await authorize(env);
  env.queue.push(reply(page(1, 1, 2)), reply(page(1, 1, 2)), reply(page(2, 2, 2)));
  await env.client.readAssets({ limit: 1 });
  const refreshed = await env.client.readAssets({ limit: 1 });
  assert.equal(refreshed.restarted, false);
  const second = await env.client.readAssets({ limit: 1, cursor: refreshed.page.progress.next_cursor });
  assert.equal(second.page.progress.range_start, '2');
  env.client.clear();
});

for (const transportConflict of [true, false]) {
  test(`snapshot restart resets ID and cursor history after ${transportConflict ? 'HTTP 409' : 'digest change'}`, async () => {
    const env = environment();
    await authorize(env);
    env.queue.push(reply(page(1, 1, 2)));
    const first = await env.client.readAssets({ limit: 1 });
    const restartedFirst = page(1, 1, 2), restartedSecond = page(2, 2, 2);
    restartedFirst.snapshot_digest = 'd'.repeat(64);
    restartedSecond.snapshot_digest = 'd'.repeat(64);
    env.queue.push(transportConflict
      ? reply({ code: 'asset_access_snapshot_changed' }, 409) : reply(restartedSecond));
    env.queue.push(reply(restartedFirst), reply(restartedSecond));
    const restarted = await env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor });
    assert.equal(restarted.restarted, true);
    assert.equal(restarted.page.progress.range_start, '1');
    assert.equal((await env.client.readAssets({ limit: 1,
      cursor: restarted.page.progress.next_cursor })).page.progress.range_start, '2');
    env.client.clear();
  });
}

test('identity deadline shortening clears old chain history before reading the same first-page IDs', async () => {
  const env = environment();
  await authorize(env);
  env.queue.push(reply(page(1, 1, 2)));
  await env.client.readAssets({ limit: 1 });
  const shorter = '2030-01-01T00:10:00Z';
  env.queue.push(reply({ ...fixture.valid.identity, expires_at: shorter }));
  await env.client.identity();
  assert.equal(env.client.state.has_snapshot, false);
  const firstPage = page(1, 1, 2), secondPage = page(2, 2, 2);
  firstPage.expires_at = shorter; secondPage.expires_at = shorter;
  env.queue.push(reply(firstPage), reply(secondPage));
  const first = await env.client.readAssets({ limit: 1 });
  assert.equal((await env.client.readAssets({ limit: 1,
    cursor: first.page.progress.next_cursor })).page.progress.range_start, '2');
  env.client.clear();
});

test('shortened expiry encountered mid-pagination restarts without rejecting previously seen IDs', async () => {
  const env = environment();
  await authorize(env);
  env.queue.push(reply(page(1, 1, 2)));
  const first = await env.client.readAssets({ limit: 1 });
  const shorterFirst = page(1, 1, 2), shorterSecond = page(2, 2, 2);
  shorterFirst.expires_at = '2030-01-01T00:10:00Z';
  shorterSecond.expires_at = shorterFirst.expires_at;
  env.queue.push(reply(shorterSecond), reply(shorterFirst), reply(shorterSecond));
  const restarted = await env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor });
  assert.equal(restarted.restarted, true);
  assert.equal((await env.client.readAssets({ limit: 1,
    cursor: restarted.page.progress.next_cursor })).page.progress.range_start, '2');
  env.client.clear();
});

test('clear and a fresh grant discard IDs from the previous authorization', async () => {
  const env = environment();
  await authorize(env);
  env.queue.push(reply(page(1, 1, 2)));
  await env.client.readAssets({ limit: 1 });
  env.client.clear();
  await authorize(env, { grant_id: `aag_${'8'.repeat(32)}`, access_token: `aat_${'8'.repeat(64)}` });
  env.queue.push(reply(page(1, 1, 2)), reply(page(2, 2, 2)));
  const first = await env.client.readAssets({ limit: 1 });
  assert.equal((await env.client.readAssets({ limit: 1,
    cursor: first.page.progress.next_cursor })).page.progress.range_start, '2');
  env.client.clear();
});

test('the complete 10000-ID budget succeeds and the 10001st ID fails without retaining assets', async () => {
  const env = environment();
  await authorize(env);
  let cursor = null;
  for (let start = 1; start <= 9981; start += 20) {
    env.queue.push(reply(page(start, start + 19, 10001)));
    const result = await env.client.readAssets({ limit: 20, cursor });
    assert.equal(result.page.progress.range_end, String(start + 19));
    cursor = result.page.progress.next_cursor;
  }
  assert.equal(env.client.state.status, 'authorized');
  assert.equal(env.client.state.has_snapshot, true);
  env.queue.push(reply(page(10001, 10001, 10001)));
  await assert.rejects(env.client.readAssets({ limit: 20, cursor }), { code: 'pagination_limit' });
  assert.equal(env.client.state.status, 'unauthenticated');
  assert.equal(env.client.state.has_snapshot, false);
  await assert.rejects(env.client.readAssets(), { code: 'authorization_required' });
});
