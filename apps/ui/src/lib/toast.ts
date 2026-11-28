import { writable } from "svelte/store";

export type ToastType = "success" | "error" | "info" | "warning";

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: string;
  message: string;
  extra?: string;
  type: ToastType;
  duration: number;
  remaining: number;
  startedAt?: number;
  paused?: boolean;
  timeoutId?: number;
  key?: number; // Used to force animation restart
  action?: ToastAction;
}

const MAX_TOASTS = 3;
const DEFAULT_DURATION = 4000;

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);

  function scheduleRemoval(id: string, ms: number): number | undefined {
    if (ms <= 0) return undefined;
    return setTimeout(() => {
      remove(id);
    }, ms) as unknown as number;
  }

  function show(
    message: string,
    type: ToastType = "info",
    extra?: string,
    duration?: number,
    action?: ToastAction,
  ) {
    // Error toasts persist until explicitly dismissed unless a positive duration is supplied.
    const resolvedDuration = duration ?? (type === "error" ? 0 : DEFAULT_DURATION);
    let id = "";

    update((toasts) => {
      const existing = toasts.find((t) => t.message === message && t.type === type && !action);

      if (existing) {
        if (existing.timeoutId) clearTimeout(existing.timeoutId);
        id = existing.id;
        const timeoutId = scheduleRemoval(id, resolvedDuration);
        return toasts.map((t) =>
          t.id === existing.id
            ? {
                ...t,
                extra,
                action,
                duration: resolvedDuration,
                remaining: resolvedDuration,
                startedAt: Date.now(),
                paused: false,
                timeoutId,
                key: Date.now(),
              }
            : t,
        );
      }

      id = `${Date.now()}-${Math.random()}`;
      const timeoutId = scheduleRemoval(id, resolvedDuration);
      const newToast: Toast = {
        id,
        message,
        extra,
        action,
        type,
        duration: resolvedDuration,
        remaining: resolvedDuration,
        startedAt: Date.now(),
        paused: false,
        timeoutId,
        key: Date.now(),
      };
      const updatedToasts = [...toasts, newToast];

      if (updatedToasts.length > MAX_TOASTS) {
        const removed = updatedToasts[0];
        if (removed.timeoutId) clearTimeout(removed.timeoutId);
        return updatedToasts.slice(-MAX_TOASTS);
      }

      return updatedToasts;
    });

    return id;
  }

  function pause(id: string) {
    update((toasts) =>
      toasts.map((t) => {
        if (t.id !== id || t.duration <= 0 || t.paused) return t;
        if (t.timeoutId) clearTimeout(t.timeoutId);
        const elapsed = t.startedAt ? Date.now() - t.startedAt : 0;
        const remaining = Math.max(400, t.remaining - elapsed);
        return { ...t, paused: true, remaining, timeoutId: undefined };
      }),
    );
  }

  function resume(id: string) {
    update((toasts) =>
      toasts.map((t) => {
        if (t.id !== id || t.duration <= 0 || !t.paused) return t;
        const timeoutId = scheduleRemoval(id, t.remaining);
        return { ...t, paused: false, startedAt: Date.now(), timeoutId };
      }),
    );
  }

  function remove(id: string) {
    update((toasts) => {
      const target = toasts.find((t) => t.id === id);
      if (target?.timeoutId) clearTimeout(target.timeoutId);
      return toasts.filter((t) => t.id !== id);
    });
  }

  function clear() {
    update((toasts) => {
      toasts.forEach((t) => {
        if (t.timeoutId) clearTimeout(t.timeoutId);
      });
      return [];
    });
  }

  return {
    subscribe,
    show,
    pause,
    resume,
    success: (message: string, duration?: number) => show(message, "success", undefined, duration),
    error: (message: string, error: string, duration?: number) => show(message, "error", error, duration),
    info: (message: string, duration?: number, action?: ToastAction) => show(message, "info", undefined, duration, action),
    warning: (message: string, duration?: number) => show(message, "warning", undefined, duration),
    remove,
    clear,
  };
}

export const toast = createToastStore();
