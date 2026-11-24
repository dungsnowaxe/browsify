import { get, writable } from "svelte/store";
import type { EditorGroup } from "../types";
import {
  align,
  equal,
  initial,
  merge,
  type Choose,
  type Draft,
  type Values,
} from "./configMerge";

export const draft = writable<Draft | null>(null);
export const draftError = writable("");
export const saving = writable(false);
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
const display = (v: unknown) =>
  v === undefined
    ? "(removed)"
    : typeof v === "string"
      ? v
      : JSON.stringify(v, null, 2);
const choose: Choose = (field, base, mine, current) => {
  const answer = window.prompt(
    `Conflict in ${field}\n\nOriginal: ${display(base)}\nDraft: ${display(mine)}\nCurrent: ${display(current)}\n\nEnter “draft” or “current”. Cancel keeps your edits for later.`,
  );
  if (answer?.trim().toLowerCase() === "draft") return mine;
  if (answer?.trim().toLowerCase() === "current") return current;
  throw new Error("Conflict unresolved. Your draft is still open.");
};
const chooseBetweenTabs: Choose = (field, original, mine, other) =>
  window.confirm(
    `Two tabs changed ${field}.\n\nOriginal: ${display(original)}\nThis tab: ${display(mine)}\nOther tab: ${display(other)}\n\nOK keeps this tab’s value. Cancel keeps the other tab’s value.`,
  )
    ? mine
    : other;

function identify(row: Values["rows"][number], candidates: Values["rows"]) {
  const describe = (r: Values["rows"][number]) =>
    r.rule
      ? `${r.rule.match.join(", ")} → ${r.rule.browser}${r.rule.profile ? " (" + r.rule.profile + ")" : ""}`
      : r.description || "Advanced rule";
  const lines = candidates.map(
    (r, i) => `${i + 1}. Original handler ${Number(r.key) + 1}: ${describe(r)}`,
  );
  const pick = window.prompt(
    `The file now contains “${describe(row)}”. Which original rule does it correspond to?\n${lines.join("\n")}\n0. A newly added rule`,
  );
  if (pick === null || !/^\d+$/.test(pick) || Number(pick) > candidates.length)
    throw new Error("Rule identity unresolved. Your draft is still open.");
  return Number(pick) === 0 ? null : candidates[Number(pick) - 1].key;
}
export async function rpc(type: string, extra: Record<string, unknown> = {}) {
  const token = window.__BROWSIFY_TOKEN__ ?? "";
  const response = await fetch("/api/rpc", {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-Browsify-Token": token },
    body: JSON.stringify({ type, ...extra }),
  });
  if (!response.ok) throw new Error("Could not reach Browsify.");
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
      draftError.set(String(e.message ?? e));
    });
  return queue;
}
function rebase(from: Draft, onto: Draft) {
  if (from.base.revision === onto.base.revision) return from.current;
  if (equal(from.original.rows, from.current.rows)) {
    return merge(
      { ...from.original, rows: onto.original.rows },
      { ...from.current, rows: onto.original.rows },
      onto.original,
      choose,
    );
  }
  const current = align(from.original.rows, onto.original.rows, identify);
  const merged = merge(
    from.original,
    from.current,
    { rows: current, settings: onto.original.settings },
    choose,
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
function combineEdits(before: Draft, local: Draft, shared: Draft): Draft {
  return {
    ...shared,
    current: merge(
      rebase(before, shared),
      rebase(local, shared),
      shared.current,
      chooseBetweenTabs,
    ),
  };
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
    try {
      const stored = read();
      // A sibling may have saved and cleared storage before its broadcast arrived.
      const committed = localStorage.getItem(key + ".revision");
      if (!stored && committed && disk?.base.revision !== committed)
        disk = await latest();
      const shared = stored ?? disk ?? before;
      const original = unpublished?.before ?? before;
      publish(combineEdits(original, local, shared));
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
  return run(async () => {
    disk = await latest();
    sync();
  });
}
export function save() {
  if (get(saving)) return;
  saving.set(true);
  void run(async () => {
    try {
      const current = await latest();
      let shared = read() ?? current;
      if (unpublished) {
        shared = combineEdits(unpublished.before, unpublished.local, shared);
        publish(shared);
        unpublished = null;
      }
      const merged = rebase(shared, current);
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
    } finally {
      saving.set(false);
      sync();
    }
  });
}
export function discard() {
  if (!window.confirm("Discard the shared draft in all Browsify tabs?")) return;
  void run(async () => {
    unpublished = null;
    localStorage.removeItem(key);
    channel.postMessage("changed");
    sync();
    disk = await latest();
    draftError.set("");
    sync();
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
    PerformanceNavigationTiming | undefined;
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
