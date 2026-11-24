import assert from 'node:assert/strict';
import { test } from 'node:test';
import { get } from 'svelte/store';
import { submitUrlTest, receiveUrlTestResult, clearUrlTest, testUrlLoading, testUrlError, testUrlSubmitted, testUrlResult } from '../src/lib/testUrlStore.ts';

test('pending tests survive view subscriptions, time out, and reject stale replies', (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const requests = [];
  globalThis.window = { finicky: { sendMessage: msg => requests.push(msg) } };
  t.after(() => { clearUrlTest(); delete globalThis.window; });
  submitUrlTest('https://first.example/');
  const unsubscribe = testUrlLoading.subscribe(() => {});
  unsubscribe();
  assert.equal(get(testUrlLoading), true);
  submitUrlTest('https://blocked.example/');
  assert.equal(requests.length, 1);
  t.mock.timers.tick(10000);
  assert.equal(get(testUrlLoading), false);
  assert.match(get(testUrlError), /did not respond/);
  submitUrlTest('https://second.example/');
  receiveUrlTestResult({ error: 'stale' }, requests[0].requestId);
  assert.equal(get(testUrlLoading), true);
  assert.equal(get(testUrlResult), null);
  assert.equal(get(testUrlSubmitted), 'https://second.example/');
  receiveUrlTestResult({ error: 'current' }, requests[1].requestId);
  assert.deepEqual(get(testUrlResult), { error: 'current' });
  assert.equal(get(testUrlLoading), false);
});
