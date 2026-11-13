import { writable } from 'svelte/store';

export interface ResolvedTestUrl {
  browser: string;
  url: string;
  openInBackground: boolean;
  profile?: string;
  warning?: string;
}

export type TestUrlResult = ResolvedTestUrl | { error: string };

export const testUrlResult = writable<TestUrlResult | null>(null);
export const testUrlInput = writable<string>('');
export const testUrlSubmitted = writable<string>('');

export const testUrlLoading = writable(false);
export const testUrlError = writable('');
let nextRequestId = 0;
let activeRequestId: number | null = null;
let timeout: ReturnType<typeof setTimeout>;

export function submitUrlTest(url: string): void {
  if (activeRequestId !== null) return;
  const requestId = ++nextRequestId;
  activeRequestId = requestId;
  testUrlSubmitted.set(url);
  testUrlResult.set(null);
  testUrlError.set('');
  testUrlLoading.set(true);
  timeout = setTimeout(() => {
    activeRequestId = null;
    testUrlLoading.set(false);
    testUrlError.set('Browsify did not respond. Check the app is running, then try again.');
  }, 10000);
  window.finicky.sendMessage({ type: 'testUrl', url, requestId });
}

export function receiveUrlTestResult(result: TestUrlResult, requestId: number): void {
  if (requestId !== activeRequestId) return;
  clearTimeout(timeout);
  activeRequestId = null;
  testUrlResult.set(result);
  testUrlLoading.set(false);
  testUrlError.set('');
}

export function clearUrlTest(): void {
  clearTimeout(timeout);
  activeRequestId = null;
  testUrlInput.set('');
  testUrlSubmitted.set('');
  testUrlResult.set(null);
  testUrlLoading.set(false);
  testUrlError.set('');
}
