export interface OpenerInfo {
  name: string;
  bundleId: string;
  path: string;
  windowTitle?: string;
}

export interface OpenUrlOptions {
  opener: OpenerInfo | null;
  originalUrl?: FinickyURL;
}

export interface FinickyURL extends URL {
  readonly urlString: string;
  readonly opener: OpenerInfo | null;
}

export type UrlMatcher =
  | string
  | RegExp
  | ((url: FinickyURL, options: OpenUrlOptions) => boolean)
  | Array<string | RegExp | ((url: FinickyURL, options: OpenUrlOptions) => boolean)>;

export interface BrowserObject {
  name: string;
  appType?: "appName" | "bundleId" | "path" | "none";
  openInBackground?: boolean;
  profile?: string;
  container?: string;
  appId?: string;
  appMode?: boolean;
  args?: string[];
}

export type BrowserResult =
  | string
  | BrowserObject
  | null
  | ((url: FinickyURL, options: OpenUrlOptions) => string | BrowserObject | null);

export interface BrowserHandler {
  match: UrlMatcher;
  browser: BrowserResult;
}

export interface RewriteRule {
  match: UrlMatcher;
  url: string | ((url: FinickyURL, options: OpenUrlOptions) => string | URL | FinickyURL);
}

export interface ConfigOptions {
  keepRunning?: boolean;
  hideIcon?: boolean;
  logRequests?: boolean;
  checkForUpdates?: boolean;
}

export interface FinickyConfig {
  defaultBrowser: BrowserResult;
  options?: ConfigOptions;
  rewrite?: RewriteRule[];
  handlers?: BrowserHandler[];
}

export type BrowsifyConfig = FinickyConfig;

export interface ModifierKeys {
  shift: boolean;
  option: boolean;
  command: boolean;
  control: boolean;
  capsLock: boolean;
  fn: boolean;
  function: boolean;
}

export interface SystemInfo {
  localizedName: string;
  name: string;
}

export interface PowerInfo {
  isCharging: boolean;
  isConnected: boolean;
  percentage?: number;
}

export interface FinickyGlobal {
  matchHostnames(matchers: string | RegExp | Array<string | RegExp>): (url: FinickyURL) => boolean;
  matchDomains(matchers: string | RegExp | Array<string | RegExp>): (url: FinickyURL) => boolean;
  matchOpener(bundleIds: string | string[]): (url: FinickyURL, options: OpenUrlOptions) => boolean;
  stripParams(patterns: string | string[]): RewriteRule;
  forceHttps(match?: UrlMatcher): RewriteRule;
  replaceHost(match: UrlMatcher, toHost: string): RewriteRule;
  getModifierKeys(): ModifierKeys;
  getSystemInfo(): SystemInfo;
  getPowerInfo(): PowerInfo;
  isAppRunning(identifier: string): boolean;
}

declare global {
  const finicky: FinickyGlobal;
}
