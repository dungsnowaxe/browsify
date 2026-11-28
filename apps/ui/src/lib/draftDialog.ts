import { writable } from "svelte/store";

export interface ConflictDialogRequest {
  kind: "conflict";
  scope: "disk" | "tab";
  field: string;
  base: string;
  mine: string;
  theirs: string;
  mineLabel: string;
  theirsLabel: string;
  step?: number;
  total?: number;
  resolve: (choice: "mine" | "theirs" | null) => void;
}

export interface IdentifyCandidate {
  key: string;
  number: number;
  summary: string;
}

export interface IdentifyDialogRequest {
  kind: "identify";
  rowSummary: string;
  candidates: IdentifyCandidate[];
  step?: number;
  resolve: (choice: string | "new" | null) => void;
}

export interface DiscardDialogRequest {
  kind: "discard";
  sharedAcrossTabs: boolean;
  resolve: (confirmed: boolean) => void;
}

export interface ConfirmImportDialogRequest {
  kind: "confirmImport";
  sourceSummary: string;
  resolve: (confirmed: boolean) => void;
}

export interface ConfirmClearLogsDialogRequest {
  kind: "confirmClearLogs";
  count: number;
  resolve: (confirmed: boolean) => void;
}

export type DraftDialogRequest =
  | ConflictDialogRequest
  | IdentifyDialogRequest
  | DiscardDialogRequest
  | ConfirmImportDialogRequest
  | ConfirmClearLogsDialogRequest;

export const activeDialog = writable<DraftDialogRequest | null>(null);

let conflictStep = 0;
let conflictTotal = 0;

export function resetConflictSteps() {
  conflictStep = 0;
  conflictTotal = 0;
}

export function setConflictTotal(total: number) {
  conflictStep = 0;
  conflictTotal = Math.max(0, total);
}

export function askConflict(
  params: Omit<ConflictDialogRequest, "kind" | "resolve" | "step" | "total">,
): Promise<"mine" | "theirs" | null> {
  const step = ++conflictStep;
  const total = conflictTotal > 0 ? conflictTotal : undefined;
  return new Promise((resolve) => {
    activeDialog.set({
      kind: "conflict",
      step,
      total,
      ...params,
      resolve: (choice) => {
        activeDialog.set(null);
        resolve(choice);
      },
    });
  });
}

export function askIdentify(
  params: Omit<IdentifyDialogRequest, "kind" | "resolve" | "step">,
): Promise<string | "new" | null> {
  const step = ++conflictStep;
  return new Promise((resolve) => {
    activeDialog.set({
      kind: "identify",
      step,
      ...params,
      resolve: (choice) => {
        activeDialog.set(null);
        resolve(choice);
      },
    });
  });
}

export function askDiscard(sharedAcrossTabs: boolean): Promise<boolean> {
  return new Promise((resolve) => {
    activeDialog.set({
      kind: "discard",
      sharedAcrossTabs,
      resolve: (confirmed) => {
        activeDialog.set(null);
        resolve(confirmed);
      },
    });
  });
}

export function askConfirmImport(sourceSummary: string): Promise<boolean> {
  return new Promise((resolve) => {
    activeDialog.set({
      kind: "confirmImport",
      sourceSummary,
      resolve: (confirmed) => {
        activeDialog.set(null);
        resolve(confirmed);
      },
    });
  });
}

export function askConfirmClearLogs(count: number): Promise<boolean> {
  return new Promise((resolve) => {
    activeDialog.set({
      kind: "confirmClearLogs",
      count,
      resolve: (confirmed) => {
        activeDialog.set(null);
        resolve(confirmed);
      },
    });
  });
}
