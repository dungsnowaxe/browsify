import assert from "node:assert/strict";
import test from "node:test";
import { align, alignAsync, initial, merge, mergeAsync } from "../src/lib/configMerge.ts";

const group = (rows) =>
  initial({
    revision: "rev",
    source: "primary",
    rows: rows.map(([match, browser]) => ({
      fingerprint: match + browser,
      description: match,
      rule: { match: [match], browser, profile: "" },
    })),
    settings: { options: {} },
  }).original;
const unexpected = () =>
  assert.fail("Independent edits should merge without a prompt");

test("an insertion before an edited handler preserves identity, order, and independent fields", () => {
  const base = group([
    ["work/*", "Safari"],
    ["other/*", "Chrome"],
  ]);
  const local = structuredClone(base);
  local.rows[0].rule.profile = "Research";
  const updated = group([
    ["news/*", "News"],
    ["work/*", "Firefox"],
    ["other/*", "Chrome"],
  ]);
  updated.rows = align(base.rows, updated.rows, unexpected);
  const result = merge(base, local, updated, unexpected);
  assert.deepEqual(
    result.rows.map((r) => [r.rule.match[0], r.rule.browser, r.rule.profile]),
    [
      ["news/*", "News", ""],
      ["work/*", "Firefox", "Research"],
      ["other/*", "Chrome", ""],
    ],
  );
});

test("overlapping browser edits present the rule, field and three competing values", () => {
  const base = group([["work/*", "Safari"]]);
  const local = structuredClone(base),
    remote = structuredClone(base);
  local.rows[0].rule.browser = "Firefox";
  remote.rows[0].rule.browser = "Chrome";
  const fields = [];
  const result = merge(base, local, remote, (...values) => {
    fields.push(values);
    return values[2];
  });
  assert.deepEqual(fields, [
    ["work/* · Browser Target", "Safari", "Firefox", "Chrome"],
  ]);
  assert.equal(result.rows[0].rule.browser, "Firefox");
});

test("ambiguous duplicate handlers require an explicit match", () => {
  const old = group([
    ["work/*", "Safari"],
    ["work/*", "Safari"],
  ]).rows;
  const changed = group([
    ["work/*", "Safari"],
    ["work/*", "Chrome"],
  ]).rows;
  const options = [];
  const rows = align(old, changed, (_row, candidates) => {
    options.push(candidates.map((r) => r.key));
    return candidates.at(-1).key;
  });
  assert.deepEqual(options[0], ["0", "1"]);
  assert.deepEqual(
    rows.map((r) => r.key),
    ["1", "0"],
  );
});

test("async conflict resolution surfaces human-readable setting labels and resolves asynchronously", async () => {
  const base = group([["work/*", "Safari"]]);
  base.settings = { defaultBrowser: "Safari", options: { checkForUpdates: true } };
  const local = structuredClone(base);
  const remote = structuredClone(base);
  local.settings.options.checkForUpdates = false;
  remote.settings.defaultBrowser = "Firefox";
  remote.settings.options.checkForUpdates = true;
  local.settings.defaultBrowser = "Chrome";

  const seenFields = [];
  const merged = await mergeAsync(base, local, remote, async (field, _b, mine) => {
    seenFields.push(field);
    return mine;
  });

  assert.deepEqual(seenFields, ["Default Browser Target"]);
  assert.equal(merged.settings.defaultBrowser, "Chrome");
  assert.equal(merged.settings.options.checkForUpdates, false);

  const aligned = await alignAsync(
    [
      { key: "0", fingerprint: "a", rule: { match: ["work/*"], browser: "Safari", profile: "" } },
      { key: "1", fingerprint: "a", rule: { match: ["work/*"], browser: "Safari", profile: "" } },
    ],
    [
      { key: "0", fingerprint: "a", rule: { match: ["work/*"], browser: "Safari", profile: "" } },
      { key: "1", fingerprint: "b", rule: { match: ["work/*"], browser: "Chrome", profile: "" } },
    ],
    async (_row, candidates) => candidates[0].key,
  );
  assert.deepEqual(
    aligned.map((r) => r.key),
    ["0", "1"],
  );
});
