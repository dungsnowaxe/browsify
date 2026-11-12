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
