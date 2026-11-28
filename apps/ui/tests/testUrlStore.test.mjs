import assert from 'node:assert/strict';
import { test } from 'node:test';
import { get } from 'svelte/store';
import {
  submitUrlTest,
  receiveUrlTestResult,
  clearUrlTest,
  clearTestHistory,
  testUrlLoading,
  testUrlError,
  testUrlSubmitted,
  testOpenerSubmitted,
  testUrlResult,
  testHistory,
} from '../src/lib/testUrlStore.ts';

test('pending tests survive view subscriptions, time out, and reject stale replies', (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const requests = [];
  globalThis.window = { finicky: { sendMessage: msg => requests.push(msg) } };
  t.after(() => { clearUrlTest(); clearTestHistory(); delete globalThis.window; });
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

test('opener payload and deduplicated history are recorded on successful tests', (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const requests = [];
  globalThis.window = { finicky: { sendMessage: msg => requests.push(msg) } };
  t.after(() => { clearUrlTest(); clearTestHistory(); delete globalThis.window; });
  submitUrlTest('https://docs.example/', 'com.tinyspeck.slackmacgap');
  assert.equal(requests[0].opener, 'com.tinyspeck.slackmacgap');
  assert.equal(get(testOpenerSubmitted), 'com.tinyspeck.slackmacgap');
  receiveUrlTestResult(
    {
      browser: 'Firefox',
      url: 'https://docs.example/',
      openInBackground: false,
      profile: 'Work',
      matchedBy: 'handler',
      matchedRuleIndex: 1,
      rewritten: false,
    },
    requests[0].requestId,
  );
  const history = get(testHistory);
  assert.equal(history.length, 1);
  assert.equal(history[0].opener, 'com.tinyspeck.slackmacgap');
  assert.equal(history[0].matchedRuleIndex, 1);
});
