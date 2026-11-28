import type { EditorGroup, Rule, ConfigPreferences } from "../types";

export interface DraftRow {
  key: string;
  rule: Rule | null;
  description?: string;
  fingerprint?: string;
}
export interface Values {
  rows: DraftRow[];
  settings: ConfigPreferences;
}
export interface Draft {
  base: EditorGroup;
  original: Values;
  current: Values;
}
export type Choose = <T>(field: string, base: T, draft: T, current: T) => T;
export type AsyncChoose = <T>(field: string, base: T, draft: T, current: T) => Promise<T>;

export const SETTING_LABELS: Record<string, string> = {
  defaultBrowser: "Default Browser Target",
  defaultProfile: "Default Profile Selector",
  keepRunning: "Keep running",
  hideIcon: "Hide menu bar icon",
  logRequests: "Log URL requests",
  checkForUpdates: "Automatic Updates",
};

export function settingLabel(key: string): string {
  return SETTING_LABELS[key] ?? key;
}

export function equal(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (!a || !b || typeof a !== "object" || typeof b !== "object") return false;
  const x = a as Record<string, unknown>,
    y = b as Record<string, unknown>;
  const keys = new Set([...Object.keys(x), ...Object.keys(y)]);
  return [...keys].every((k) => equal(x[k], y[k]));
}

export function initial(group: EditorGroup): Draft {
  const rows = group.rows.map((r, i) => ({
    key: String(i),
    description: r.description,
    fingerprint: r.fingerprint,
    rule: r.rule
      ? {
          browser: r.rule.browser,
          profile: r.rule.profile ?? "",
          match: [r.rule.match].flat(),
        }
      : null,
  }));
  const original = { rows, settings: group.settings ?? {} };
  return { base: group, original, current: structuredClone(original) };
}

function scalar<T>(
  field: string,
  base: T,
  draft: T,
  current: T,
  choose: (field: string, base: T, draft: T, current: T) => T,
): T {
  if (equal(draft, base) || equal(draft, current)) return current;
  return equal(current, base) ? draft : choose(field, base, draft, current);
}
function mergeRow(
  base: DraftRow,
  mine: DraftRow,
  theirs: DraftRow,
  choose: Choose,
): DraftRow {
  const label =
    base.rule?.match.join(", ") || base.description || "Advanced Handler Rule";
  if (!base.rule || !mine.rule || !theirs.rule)
    return scalar(label, base, mine, theirs, choose);
  const b = base.rule,
    m = mine.rule,
    t = theirs.rule;
  return {
    ...theirs,
    rule: {
      match: scalar(
        label + " · URL pattern",
        b.match,
        m.match,
        t.match,
        choose,
      ),
      browser: scalar(
        label + " · Browser Target",
        b.browser,
        m.browser,
        t.browser,
        choose,
      ),
      profile: scalar(
        label + " · Profile Selector",
        b.profile,
        m.profile,
        t.profile,
        choose,
      ),
    },
  };
}
export function merge(
  base: Values,
  mine: Values,
  theirs: Values,
  choose: Choose,
): Values {
  const b = base.settings,
    m = mine.settings,
    t = theirs.settings;
  const settings: ConfigPreferences = { options: {} };
  for (const k of ["defaultBrowser", "defaultProfile"] as const) {
    settings[k] = scalar(settingLabel(k), b[k], m[k], t[k], choose);
  }
  for (const k of [
    "keepRunning",
    "hideIcon",
    "logRequests",
    "checkForUpdates",
  ] as const) {
    settings.options![k] = scalar(
      settingLabel(k),
      b.options?.[k],
      m.options?.[k],
      t.options?.[k],
      choose,
    );
  }
  const index = (v: Values) => new Map(v.rows.map((r) => [r.key, r]));
  const bi = index(base),
    mi = index(mine),
    ti = index(theirs);
  const rows = new Map<string, DraftRow>();
  for (const key of new Set([...bi.keys(), ...ti.keys(), ...mi.keys()])) {
    const br = bi.get(key),
      mr = mi.get(key),
      tr = ti.get(key);
    const row =
      br && mr && tr
        ? mergeRow(br, mr, tr, choose)
        : scalar(
            br?.description || mr?.rule?.match.join(", ") || "Handler Rule",
            br,
            mr,
            tr,
            choose,
          );
    if (row) rows.set(key, row);
  }
  // Compare the relative order of surviving original rows. Insertions can
  // then be placed next to their nearest surviving neighbor from each side.
  const common = (v: Values) =>
    v.rows
      .map((r) => r.key)
      .filter((k) => bi.has(k) && mi.has(k) && ti.has(k) && rows.has(k));
  const order = scalar(
    "Handler Rule order",
    common(base),
    common(mine),
    common(theirs),
    (_field, b, m, t) => {
      const list = (keys: string[]) =>
        keys.map((k) => [
          k,
          bi.get(k)?.description || bi.get(k)?.rule?.match.join(", "),
        ]);
      return choose("Handler Rule order", list(b), list(m), list(t)).map(
        ([key]) => key!,
      );
    },
  );
  const ordered = [...order];
  for (const v of [theirs, mine]) {
    let previous: string | undefined;
    for (const [i, row] of v.rows.entries()) {
      if (!rows.has(row.key)) continue;
      if (!ordered.includes(row.key)) {
        const next = v.rows.slice(i + 1).find((r) => ordered.includes(r.key));
        const position = previous
          ? ordered.indexOf(previous) + 1
          : next
            ? ordered.indexOf(next.key)
            : ordered.length;
        ordered.splice(position, 0, row.key);
      }
      previous = row.key;
    }
  }
  return { settings, rows: ordered.map((key) => rows.get(key)!) };
}

// IDs are positions within one disk revision. Match the latest rows back to
// the draft before using those positions to write into the latest document.
export function align(
  base: DraftRow[],
  disk: DraftRow[],
  identify: (row: DraftRow, candidates: DraftRow[]) => string | null,
): DraftRow[] {
  if (
    equal(
      base.map((r) => r.fingerprint),
      disk.map((r) => r.fingerprint),
    )
  )
    return disk.map((r, i) => ({ ...r, key: base[i].key }));
  const result = disk.map((r) => ({ ...r, key: "" }));
  const free = new Map(base.map((r) => [r.key, r]));
  const match = (i: number, key: string) => {
    result[i].key = key;
    free.delete(key);
  };
  for (const signature of [
    (r: DraftRow) => r.fingerprint,
    (r: DraftRow) => (r.rule ? JSON.stringify(r.rule.match) : undefined),
  ]) {
    for (let i = 0; i < disk.length; i++) {
      if (result[i].key) continue;
      const sig = signature(disk[i]);
      if (!sig) continue;
      const peers = result.filter((r) => !r.key && signature(r) === sig);
      const candidates = [...free.values()].filter((r) => signature(r) === sig);
      if (peers.length === 1 && candidates.length === 1)
        match(i, candidates[0].key);
    }
  }
  for (let i = 0; i < result.length; i++) {
    if (result[i].key) continue;
    const candidates = [...free.values()];
    const key = candidates.length ? identify(disk[i], candidates) : null;
    if (key != null && !free.has(key))
      throw new Error("Choose one of the listed original rules.");
    match(i, key ?? crypto.randomUUID());
  }
  return result;
}

async function scalarAsync<T>(
  field: string,
  base: T,
  draft: T,
  current: T,
  choose: (field: string, base: T, draft: T, current: T) => Promise<T>,
): Promise<T> {
  if (equal(draft, base) || equal(draft, current)) return current;
  return equal(current, base) ? draft : choose(field, base, draft, current);
}

async function mergeRowAsync(
  base: DraftRow,
  mine: DraftRow,
  theirs: DraftRow,
  choose: AsyncChoose,
): Promise<DraftRow> {
  const label =
    base.rule?.match.join(", ") || base.description || "Advanced Handler Rule";
  if (!base.rule || !mine.rule || !theirs.rule)
    return scalarAsync(label, base, mine, theirs, choose);
  const b = base.rule,
    m = mine.rule,
    t = theirs.rule;
  return {
    ...theirs,
    rule: {
      match: await scalarAsync(
        label + " · URL pattern",
        b.match,
        m.match,
        t.match,
        choose,
      ),
      browser: await scalarAsync(
        label + " · Browser Target",
        b.browser,
        m.browser,
        t.browser,
        choose,
      ),
      profile: await scalarAsync(
        label + " · Profile Selector",
        b.profile,
        m.profile,
        t.profile,
        choose,
      ),
    },
  };
}

export function countMergeConflicts(
  base: Values,
  mine: Values,
  theirs: Values,
): number {
  let count = 0;
  merge(base, mine, theirs, (_field, _b, m) => {
    count++;
    return m;
  });
  return count;
}

export async function mergeAsync(
  base: Values,
  mine: Values,
  theirs: Values,
  choose: AsyncChoose,
): Promise<Values> {
  const b = base.settings,
    m = mine.settings,
    t = theirs.settings;
  const settings: ConfigPreferences = { options: {} };
  for (const k of ["defaultBrowser", "defaultProfile"] as const) {
    settings[k] = await scalarAsync(settingLabel(k), b[k], m[k], t[k], choose);
  }
  for (const k of [
    "keepRunning",
    "hideIcon",
    "logRequests",
    "checkForUpdates",
  ] as const) {
    settings.options![k] = await scalarAsync(
      settingLabel(k),
      b.options?.[k],
      m.options?.[k],
      t.options?.[k],
      choose,
    );
  }
  const index = (v: Values) => new Map(v.rows.map((r) => [r.key, r]));
  const bi = index(base),
    mi = index(mine),
    ti = index(theirs);
  const rows = new Map<string, DraftRow>();
  for (const key of new Set([...bi.keys(), ...ti.keys(), ...mi.keys()])) {
    const br = bi.get(key),
      mr = mi.get(key),
      tr = ti.get(key);
    const row =
      br && mr && tr
        ? await mergeRowAsync(br, mr, tr, choose)
        : await scalarAsync(
            br?.description || mr?.rule?.match.join(", ") || "Handler Rule",
            br,
            mr,
            tr,
            choose,
          );
    if (row) rows.set(key, row);
  }
  const common = (v: Values) =>
    v.rows
      .map((r) => r.key)
      .filter((k) => bi.has(k) && mi.has(k) && ti.has(k) && rows.has(k));
  const order = await scalarAsync(
    "Handler Rule order",
    common(base),
    common(mine),
    common(theirs),
    async (_field, bKeys, mKeys, tKeys) => {
      const list = (keys: string[]) =>
        keys.map((k) => [
          k,
          bi.get(k)?.description || bi.get(k)?.rule?.match.join(", "),
        ]);
      const chosen = await choose(
        "Handler Rule order",
        list(bKeys),
        list(mKeys),
        list(tKeys),
      );
      return chosen.map(([key]) => key!);
    },
  );
  const ordered = [...order];
  for (const v of [theirs, mine]) {
    let previous: string | undefined;
    for (const [i, row] of v.rows.entries()) {
      if (!rows.has(row.key)) continue;
      if (!ordered.includes(row.key)) {
        const next = v.rows.slice(i + 1).find((r) => ordered.includes(r.key));
        const position = previous
          ? ordered.indexOf(previous) + 1
          : next
            ? ordered.indexOf(next.key)
            : ordered.length;
        ordered.splice(position, 0, row.key);
      }
      previous = row.key;
    }
  }
  return { settings, rows: ordered.map((key) => rows.get(key)!) };
}

export async function alignAsync(
  base: DraftRow[],
  disk: DraftRow[],
  identify: (row: DraftRow, candidates: DraftRow[]) => Promise<string | null>,
): Promise<DraftRow[]> {
  if (
    equal(
      base.map((r) => r.fingerprint),
      disk.map((r) => r.fingerprint),
    )
  )
    return disk.map((r, i) => ({ ...r, key: base[i].key }));
  const result = disk.map((r) => ({ ...r, key: "" }));
  const free = new Map(base.map((r) => [r.key, r]));
  const match = (i: number, key: string) => {
    result[i].key = key;
    free.delete(key);
  };
  for (const signature of [
    (r: DraftRow) => r.fingerprint,
    (r: DraftRow) => (r.rule ? JSON.stringify(r.rule.match) : undefined),
  ]) {
    for (let i = 0; i < disk.length; i++) {
      if (result[i].key) continue;
      const sig = signature(disk[i]);
      if (!sig) continue;
      const peers = result.filter((r) => !r.key && signature(r) === sig);
      const candidates = [...free.values()].filter((r) => signature(r) === sig);
      if (peers.length === 1 && candidates.length === 1)
        match(i, candidates[0].key);
    }
  }
  for (let i = 0; i < result.length; i++) {
    if (result[i].key) continue;
    const candidates = [...free.values()];
    const key = candidates.length ? await identify(disk[i], candidates) : null;
    if (key != null && !free.has(key))
      throw new Error("Choose one of the listed original rules.");
    match(i, key ?? crypto.randomUUID());
  }
  return result;
}
