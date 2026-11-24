import assert from 'node:assert/strict';
import { test } from 'node:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createChromeProfile } from '../../../.agents/skills/design-taste-frontend/scripts/chrome-profile.mjs';

test('profile cleanup preserves existing contents and parallel profiles', () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'profile-test-'));
  try {
    fs.writeFileSync(path.join(base, 'sentinel'), 'keep');
    const first = createChromeProfile(base);
    const second = createChromeProfile(base);
    assert.notEqual(first.directory, second.directory);
    assert.equal(path.dirname(first.directory), base);
    first.cleanup();
    assert.equal(fs.readFileSync(path.join(base, 'sentinel'), 'utf8'), 'keep');
    assert.equal(fs.existsSync(second.directory), true);
    second.cleanup();
  } finally { fs.rmSync(base, { recursive: true, force: true }); }
});

test('project source directory cannot be selected as the profile root', () => {
  const profile = createChromeProfile(path.join(process.cwd(), 'src'));
  try {
    assert.equal(path.dirname(profile.directory), os.tmpdir());
    assert.equal(fs.existsSync('src/App.svelte'), true);
  } finally { profile.cleanup(); }
});
