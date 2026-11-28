import { get, writable } from "svelte/store";
import type { EditorGroup } from "../types";
import {
  alignAsync,
  countMergeConflicts,
  equal,
  initial,
  mergeAsync,
  type AsyncChoose,
  type Draft,
  type DraftRow,
  type Values,
} from "./configMerge";
import {
  askConflict,
  askDiscard,
  askIdentify,
  resetConflictSteps,
  setConflictTotal,
} from "./draftDialog";
import { toast } from "./toast";

export const draft = writable<Draft | null>(null);
export const draftError = writable("");
export const draftLoading = writable(true);
export const saving = writable(false);
export const validationRequested = writable(0);

export type PendingRuleFocus =
  | { kind: "invalid" }
  | { kind: "index"; index: number };

export const pendingRuleFocus = writable<PendingRuleFocus | null>(null);

export function requestFocusInvalidRule() {
  validationRequested.update((n) => n + 1);
  pendingRuleFocus.set({ kind: "invalid" });
}

export function requestFocusRuleIndex(index: number) {
  pendingRuleFocus.set({ kind: "index", index });
}

export function consumePendingRuleFocus() {
  pendingRuleFocus.set(null);
}

export function countInvalidRules(rows: DraftRow[]): number {
  let invalid = 0;
  for (const row of rows) {
    if (!row.rule) continue;
    const missingBrowser = !row.rule.browser.trim();
    const missingPattern = row.rule.match.every((p) => !p.trim());
    if (missingBrowser || missingPattern) invalid++;
  }
  return invalid;
}

const session = window.__BROWSIFY_SESSION__ ?? location.origin;
const key = "browsify.config-draft." + session;
const members = key + ".tab.";
const tab = crypto.randomUUID();
const channel = new BroadcastChannel(key);
const read = (): Draft | null =>
  JSON.parse(localStorage.getItem(key) ?? "null");
const dirty = (v: Draft | null) => !!v && !equal(v.current, v.original);
const otherTabs = () =>
  Object.keys(localStorage).some(
    (k) => k.startsWith(members) && k !== members + tab,
  );
let disk: Draft | null = null;
let pending = 0;
let unpublished: { before: Draft; local: Draft } | null = null;
let queue = Promise.resolve();

function describeRow(row: DraftRow): string {
  if (!row.rule) return row.description || "Advanced Handler Rule";
  const target =
    row.rule.browser === "none"
      ? "Don’t open"
      : `${row.rule.browser || "No browser"}${row.rule.profile ? ` (${row.rule.profile})` : ""}`;
  return `${row.rule.match.filter(Boolean).join(", ") || "No pattern"} → ${target}`;
}

function formatValue(value: unknown): string {
  if (value === undefined) return "Removed";
  if (value === null || value === "") return "Not set";
  if (typeof value === "boolean") return value ? "On" : "Off";
  if (typeof value === "string") return value === "none" ? "Don’t open" : value;
  if (Array.isArray(value)) {
    if (!value.length) return "None";
    if (value.every((entry) => typeof entry === "string")) {
      return value.join(", ");
    }
    if (value.every((entry) => Array.isArray(entry))) {
      return value
        .map((entry, idx) => `${idx + 1}. ${String(entry[1] || "Handler Rule")}`)
        .join("\n");
    }
  }
  if (typeof value === "object" && "key" in (value as Record<string, unknown>)) {
    return describeRow(value as DraftRow);
  }
  return String(value);
}

const chooseDisk: AsyncChoose = async (field, base, mine, current) => {
  const decision = await askConflict({
    scope: "disk",
    field,
    base: formatValue(base),
    mine: formatValue(mine),
    theirs: formatValue(current),
    mineLabel: "Your draft",
    theirsLabel: "Primary Config",
  });
  if (decision === "mine") return mine;
  if (decision === "theirs") return current;
  throw new Error("Conflict left unresolved. Your unsaved draft is still open.");
};

const chooseBetweenTabs: AsyncChoose = async (field, original, mine, other) => {
  const decision = await askConflict({
    scope: "tab",
    field,
    base: formatValue(original),
    mine: formatValue(mine),
    theirs: formatValue(other),
    mineLabel: "This tab",
    theirsLabel: "Other tab",
  });
  if (decision === "mine") return mine;
  if (decision === "theirs") return other;
  throw new Error("Tab conflict left unresolved. Your edits in this tab are still open.");
};

async function identify(
  row: Values["rows"][number],
  candidates: Values["rows"],
): Promise<string | null> {
  const pick = await askIdentify({
    rowSummary: describeRow(row),
    candidates: candidates.map((r) => ({
      key: r.key,
      number: Number(r.key) + 1,
      summary: describeRow(r),
    })),
  });
  if (pick === null) {
    throw new Error("Rule match left unresolved. Your unsaved draft is still open.");
  }
  return pick === "new" ? null : pick;
}

function getToken(): string {
  if (window.__BROWSIFY_TOKEN__) return window.__BROWSIFY_TOKEN__;
  const params = new URLSearchParams(window.location.search);
  return params.get("token") ?? "";
}

export async function rpc(type: string, extra: Record<string, unknown> = {}) {
  const token = getToken();
  const response = await fetch(`/api/rpc?token=${encodeURIComponent(token)}`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-Browsify-Token": token },
    body: JSON.stringify({ type, ...extra }),
  });
  if (!response.ok) throw new Error("Could not reach Browsify. Check that the app is running and try again.");
  const reply = await response.json();
  if (reply.type?.endsWith("Error")) throw new Error(reply.message.error);
  return reply;
}

async function latest() {
  const reply = await rpc("getRuleEditor");
  const group: EditorGroup = reply.message.groups[0];
  return initial(group);
}

function sync() {
  if (!pending && !get(saving)) draft.set(unpublished?.local ?? read() ?? disk);
}

function publish(value: Draft) {
  if (dirty(value)) localStorage.setItem(key, JSON.stringify(value));
  else {
    localStorage.removeItem(key);
    if (value.base.revision)
      localStorage.setItem(key + ".revision", value.base.revision);
  }
  channel.postMessage("changed");
}

function run(work: () => Promise<void>) {
  queue = queue
    .then(() => navigator.locks.request(key, work))
    .catch((e) => {
      const message = e instanceof Error ? e.message : String(e);
      draftError.set(message.replace(/^Error:\s*/, ""));
    });
  return queue;
}

async function rebase(from: Draft, onto: Draft): Promise<Values> {
  if (from.base.revision === onto.base.revision) return from.current;
  if (equal(from.original.rows, from.current.rows)) {
    const base = { ...from.original, rows: onto.original.rows };
    const mine = { ...from.current, rows: onto.original.rows };
    setConflictTotal(countMergeConflicts(base, mine, onto.original));
    return mergeAsync(base, mine, onto.original, chooseDisk);
  }
  const current = await alignAsync(from.original.rows, onto.original.rows, identify);
  const theirs = { rows: current, settings: onto.original.settings };
  setConflictTotal(countMergeConflicts(from.original, from.current, theirs));
  const merged = await mergeAsync(
    from.original,
    from.current,
    theirs,
    chooseDisk,
  );
  const indexes = new Map(
    current.map((r, i) => [r.key, onto.original.rows[i].key]),
  );
  return {
    ...merged,
    rows: merged.rows.map((r) => ({
      ...r,
      key: indexes.get(r.key) ?? "added-" + r.key,
    })),
  };
}

async function combineEdits(
  before: Draft,
  local: Draft,
  shared: Draft,
): Promise<Draft> {
  const baseValues = await rebase(before, shared);
  const localValues = await rebase(local, shared);
  setConflictTotal(countMergeConflicts(baseValues, localValues, shared.current));
  return {
    ...shared,
    current: await mergeAsync(
      baseValues,
      localValues,
      shared.current,
      chooseBetweenTabs,
    ),
  };
}

export function clearDraftError() {
  draftError.set("");
}

export function edit(change: (value: Values) => void) {
  const before = get(draft);
  if (!before || get(saving)) return;
  const after = structuredClone(before.current);
  change(after);
  if (equal(after, before.current)) return;
  const local = { ...before, current: after };
  draft.set(local);
  pending++;
  void run(async () => {
    resetConflictSteps();
    try {
      const stored = read();
      // A sibling may have saved and cleared storage before its broadcast arrived.
      const committed = localStorage.getItem(key + ".revision");
      if (!stored && committed && disk?.base.revision !== committed)
        disk = await latest();
      const shared = stored ?? disk ?? before;
      const original = unpublished?.before ?? before;
      publish(await combineEdits(original, local, shared));
      unpublished = null;
      draftError.set("");
    } catch (e) {
      // Keep the local side available if the user cancels a cross-tab decision.
      unpublished = { before: unpublished?.before ?? before, local };
      draft.set(local);
      throw e;
    } finally {
      pending--;
    }
    sync();
  });
}

export function refresh() {
  draftLoading.set(true);
  return run(async () => {
    try {
      disk = await latest();
      draftError.set("");
      sync();
    } finally {
      draftLoading.set(false);
    }
  });
}

export function save(): Promise<boolean> {
  if (get(saving)) return Promise.resolve(false);
  const currentDraft = get(draft);
  if (currentDraft && countInvalidRules(currentDraft.current.rows) > 0) {
    validationRequested.update((n) => n + 1);
    draftError.set("Complete every Handler Rule’s Browser Target and URL pattern before saving.");
    return Promise.resolve(false);
  }
  saving.set(true);
  let ok = false;
  return run(async () => {
    resetConflictSteps();
    try {
      const current = await latest();
      let shared = read() ?? current;
      if (unpublished) {
        shared = await combineEdits(unpublished.before, unpublished.local, shared);
        publish(shared);
        unpublished = null;
      }
      const merged = await rebase(shared, current);
      const rows = equal(merged.rows, current.original.rows)
        ? undefined
        : merged.rows
            .map((r) => ({
              id: current.original.rows.findIndex((x) => x.key === r.key),
              rule: r.rule,
            }))
            .map((r) => ({ ...r, id: r.id < 0 ? null : r.id }));
      const reply = await rpc("saveRuleEditor", {
        payload: {
          revision: current.base.revision,
          settings: merged.settings,
          rows,
        },
      });
      disk = initial(reply.message.groups[0]);
      publish(disk);
      draftError.set("");
      toast.success("Configuration saved and reloaded");
      ok = true;
    } finally {
      saving.set(false);
      sync();
    }
  }).then(() => ok);
}

export function discard() {
  void askDiscard(otherTabs()).then((confirmed) => {
    if (!confirmed) return;
    void run(async () => {
      unpublished = null;
      localStorage.removeItem(key);
      channel.postMessage("changed");
      sync();
      disk = await latest();
      draftError.set("");
      sync();
      toast.info("Unsaved changes discarded");
    });
  });
}

async function changed() {
  if (localStorage.getItem(key)) sync();
  else await refresh();
}

export function startDraft() {
  // The browser releases a tab's lock even after a crash. Prune stale markers
  // on entry and focus so a dead tab cannot suppress the last-tab warning.
  let release: (() => void) | undefined;
  const join = () => {
    localStorage.setItem(members + tab, "open");
    void navigator.locks.request(
      members + tab,
      () =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
  };
  const prune = async () => {
    const snapshot = Object.keys(localStorage).filter((k) =>
      k.startsWith(members),
    );
    const locks = await navigator.locks.query();
    const live = new Set(
      [...(locks.held ?? []), ...(locks.pending ?? [])].map((l) => l.name),
    );
    for (const k of snapshot)
      if (k !== members + tab && !live.has(k)) localStorage.removeItem(k);
  };
  join();
  void prune();
  window.addEventListener("focus", prune);
  const nav = performance.getEntriesByType("navigation")[0] as
    | PerformanceNavigationTiming
    | undefined;
  const previous = sessionStorage.getItem(key);
  if (nav?.type === "reload" && previous && !localStorage.getItem(key))
    localStorage.setItem(key, previous);
  sessionStorage.removeItem(key);
  sync();
  channel.onmessage = changed;
  const before = (e: BeforeUnloadEvent) => {
    if (
      (dirty(get(draft)) || pending || localStorage.getItem(key)) &&
      !otherTabs()
    ) {
      e.preventDefault();
      e.returnValue = "";
    }
  };
  const leave = () => {
    release?.();
    localStorage.removeItem(members + tab);
    if (!otherTabs()) {
      const value = get(draft);
      if (dirty(value)) sessionStorage.setItem(key, JSON.stringify(value));
      localStorage.removeItem(key);
    }
  };
  const returnToPage = (e: PageTransitionEvent) => {
    if (e.persisted) {
      join();
      void prune();
      void refresh();
    }
  };
  window.addEventListener("beforeunload", before);
  window.addEventListener("pagehide", leave);
  window.addEventListener("pageshow", returnToPage);
  void refresh();
  return () => {
    leave();
    window.removeEventListener("focus", prune);
    window.removeEventListener("beforeunload", before);
    window.removeEventListener("pagehide", leave);
    window.removeEventListener("pageshow", returnToPage);
    channel.close();
  };
}
