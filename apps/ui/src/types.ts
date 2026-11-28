export interface Rule {
  match: string[];
  browser: string;
  profile?: string;
}

export interface Choice { id: string; label: string; aliases: string[] }
export interface EditorRow { fingerprint?: string; id?: number | null; rule?: Rule | null; description?: string }
export interface EditorGroup { settings?: ConfigPreferences; editableRows?: boolean; source: "primary"; revision?: string | null; rows: EditorRow[]; rewrites?: string[] }
export interface ProfileChoices { profiles: Choice[]; error?: boolean }
export type LoginItemStatus = "unavailable" | "off" | "on" | "needsApproval";

export interface ConfigOptions {
  keepRunning: boolean;
  hideIcon: boolean;
  logRequests: boolean;
  checkForUpdates: boolean;
}

export interface ConfigPreferences {
  defaultBrowser?: string;
  defaultProfile?: string;
  options?: Partial<ConfigOptions>;
}

export interface LogEntry {
  level: string;
  msg: string;
  time: string;
  error?: string;
  [key: string]: any; // Allow for additional dynamic fields
}

declare global {
  interface Window {
    __BROWSIFY_SESSION__?: string;
    __BROWSIFY_TOKEN__?: string;
    finicky: {
      sendMessage: (msg: any) => void;
      receiveMessage: (msg: any) => void;
      /** Stub queue populated by the WKUserScript before the Svelte app is ready */
      _queue?: any[];
    };
    webkit?: {
      messageHandlers?: {
        finicky?: {
          postMessage: (msg: string) => void;
        };
      };
    };
  }
}

export interface UpdateInfo {
  version: string;
  hasUpdate: boolean;
  updateCheckEnabled: boolean;
  downloadUrl: string;
  releaseUrl: string;
  status: "disabled" | "checking" | "upToDate" | "available" | "downloading" | "ready" | "skipped" | "deferred" | "error";
  error?: string | null;
}

export interface ConfigInfo {
  configPath: string;
  hasPrimaryConfig?: boolean;
  explicitDefaultBrowser?: boolean;
  format?: string;
  isDefaultBrowser?: boolean;
  handlers?: number;
  rewrites?: number;
  defaultBrowser?: string;
  defaultProfile?: string;
  options?: Partial<ConfigOptions>;
}
