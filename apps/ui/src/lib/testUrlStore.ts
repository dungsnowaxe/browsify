import { writable } from 'svelte/store';

export interface ResolvedTestUrl {
  browser: string;
  url: string;
  openInBackground: boolean;
  profile?: string;
  warning?: string;
  action?: { kind: "open_application" } | { kind: "dia_profile"; profile: string };
  matchedBy?: "handler" | "default";
  matchedRuleIndex?: number | null;
  rewritten?: boolean;
  opener?: string | null;
}

export type TestUrlResult = ResolvedTestUrl | { error: string };

export interface TestHistoryEntry {
  url: string;
  opener?: string;
  browser: string;
  profile?: string;
  matchedBy?: "handler" | "default";
  matchedRuleIndex?: number | null;
}

export const testUrlResult = writable<TestUrlResult | null>(null);
export const testUrlInput = writable<string>('');
export const testUrlSubmitted = writable<string>('');
export const testOpenerInput = writable<string>('');
export const testOpenerSubmitted = writable<string>('');
export const testHistory = writable<TestHistoryEntry[]>([]);

export const testUrlLoading = writable(false);
export const testUrlError = writable('');
let nextRequestId = 0;
let activeRequestId: number | null = null;
let activeRequestMeta: { url: string; opener: string } | null = null;
let timeout: ReturnType<typeof setTimeout>;

export function submitUrlTest(url: string, opener = ''): void {
  if (activeRequestId !== null) return;
  const trimmedOpener = opener.trim();
  const requestId = ++nextRequestId;
  activeRequestId = requestId;
  activeRequestMeta = { url, opener: trimmedOpener };
  testUrlSubmitted.set(url);
  testOpenerSubmitted.set(trimmedOpener);
  testUrlResult.set(null);
  testUrlError.set('');
  testUrlLoading.set(true);
  timeout = setTimeout(() => {
    activeRequestId = null;
    activeRequestMeta = null;
    testUrlLoading.set(false);
    testUrlError.set('Browsify did not respond. Check the app is running, then try again.');
  }, 10000);
  const payload: Record<string, unknown> = { type: 'testUrl', url, requestId };
  if (trimmedOpener) payload.opener = trimmedOpener;
  window.finicky.sendMessage(payload);
}

export function receiveUrlTestResult(result: TestUrlResult, requestId: number): void {
  if (requestId !== activeRequestId) return;
  clearTimeout(timeout);
  const meta = activeRequestMeta;
  activeRequestId = null;
  activeRequestMeta = null;
  testUrlResult.set(result);
  testUrlLoading.set(false);
  testUrlError.set('');
  if (meta && !('error' in result)) {
    testHistory.update((items) => {
      const next: TestHistoryEntry = {
        url: meta.url,
        opener: meta.opener || undefined,
        browser: result.browser,
        profile: result.profile || undefined,
        matchedBy: result.matchedBy,
        matchedRuleIndex: result.matchedRuleIndex,
      };
      const filtered = items.filter(
        (entry) => !(entry.url === next.url && (entry.opener ?? '') === (next.opener ?? '')),
      );
      return [next, ...filtered].slice(0, 5);
    });
  }
}

export function clearUrlTest(): void {
  clearTimeout(timeout);
  activeRequestId = null;
  activeRequestMeta = null;
  testUrlInput.set('');
  testUrlSubmitted.set('');
  testOpenerSubmitted.set('');
  testUrlResult.set(null);
  testUrlLoading.set(false);
  testUrlError.set('');
}

export function clearTestHistory(): void {
  testHistory.set([]);
}
