import test from 'node:test';
import assert from 'node:assert/strict';
import { startVisiblePolling } from '../frontend/visible-poll.ts';

const settle = async () => { await Promise.resolve(); await Promise.resolve(); };
function harness(initial = 'visible') {
  const visibility = new EventTarget();
  visibility.visibilityState = initial;
  const timers = new Map(); let id = 0;
  return {
    options: { visibility, onError: error => { throw error; },
      schedule: callback => { timers.set(++id, callback); return id; },
      cancel: timer => timers.delete(timer) },
    timers,
    change(state) { visibility.visibilityState = state; visibility.dispatchEvent(new Event('visibilitychange')); },
    fire() { const [key, callback] = timers.entries().next().value; timers.delete(key); callback(); },
  };
}
test('suspends hidden windows and refreshes immediately on return', async () => {
  const h = harness('hidden'); let calls = 0;
  const stop = startVisiblePolling(async () => { calls++; }, h.options);
  assert.equal(calls, 0); assert.equal(h.timers.size, 0);
  h.change('visible'); await settle();
  assert.equal(calls, 1); assert.equal(h.timers.size, 1);
  h.change('hidden'); assert.equal(h.timers.size, 0);
  h.change('visible'); await settle(); assert.equal(calls, 2);
  stop(); assert.equal(h.timers.size, 0);
  h.change('visible'); assert.equal(calls, 2);
});
test('slow IPC stays single-flight even across hide/restore', async () => {
  const h = harness(); let complete; let calls = 0;
  const stop = startVisiblePolling(() => { calls++; return new Promise(resolve => { complete = resolve; }); }, h.options);
  h.change('hidden'); h.change('visible'); h.change('visible');
  assert.equal(calls, 1); assert.equal(h.timers.size, 0);
  complete(); await settle(); assert.equal(calls, 2);
  stop(); complete(); await settle(); assert.equal(h.timers.size, 0);
});
test('reports failures, retries, and cannot reschedule after cleanup', async () => {
  const h = harness(); const errors = [];
  const stop = startVisiblePolling(async () => { throw new Error('IPC unavailable'); }, {...h.options, onError: e => errors.push(e.message)});
  await settle(); assert.deepEqual(errors, ['IPC unavailable']);
  assert.equal(h.timers.size, 1); h.fire(); await settle();
  assert.equal(errors.length, 2); stop(); assert.equal(h.timers.size, 0);
});
