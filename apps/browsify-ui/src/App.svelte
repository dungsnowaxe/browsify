<script lang="ts">
  import { attrs, materials } from "./design-system/styles";
  import { Router, Route, Link } from "svelte-routing";
  import LogViewer from "./pages/LogViewer.svelte";
  import StartPage from "./pages/StartPage.svelte";
  import Preferences from "./pages/Preferences.svelte";
  import TabBar from "./components/TabBar.svelte";
  import About from "./pages/About.svelte";
  import TestUrl from "./pages/TestUrl.svelte";
  import Rules from "./pages/Rules.svelte";
  import ToastContainer from "./components/ToastContainer.svelte";
  import AppearanceMenu from "./components/AppearanceMenu.svelte";
  import type { LogEntry, UpdateInfo, ConfigInfo, RulesFile } from "./types";
  import { testUrlResult } from "./lib/testUrlStore";
  import { toast } from "./lib/toast";

  function basename(path: string): string {
    return path.split("/").pop() || path;
  }

  function showPathToast(label: string, description: string, path: string) {
    toast.show(label, "info", `${description}\n${path}`, 5000);
  }

  let version = "v0.0.0";
  let buildInfo = "dev";

  let scrollArea: HTMLDivElement;
  let hasConfig = false;
  let config: ConfigInfo = { configPath: "" };
  let messageBuffer: LogEntry[] = [];
  let updateInfo: UpdateInfo | null = null;
  let rulesFile: RulesFile = { defaultBrowser: "", rules: [] };
  let installedBrowsers: string[] = [];
  let profilesByBrowser: Record<string, string[]> = {};

  $: configPaths = [...new Set([config.configPath, rulesFile.path].filter((path): path is string => !!path))];

  $: numErrors = messageBuffer.filter(
    (msg) => msg.level.toLowerCase() === "error"
  ).length;

  function handleMessage(msg: any) {
    const parsedMsg = typeof msg === "string" ? JSON.parse(msg) : msg;
    console.log("Received message:", parsedMsg.type, parsedMsg);
    switch (parsedMsg.type) {
      case "version":
        version = parsedMsg.message;
        break;
      case "buildInfo":
        buildInfo = parsedMsg.message;
        break;
      case "config":
        hasConfig = true;
        if (parsedMsg.message) {
          config = parsedMsg.message;
        }
        break;
      case "updateInfo":
        updateInfo = parsedMsg.message;
        break;
      case "testUrlResult":
        testUrlResult.set(parsedMsg.message);
        break;
      case "rules":
        rulesFile = parsedMsg.message;
        break;
      case "installedBrowsers":
        installedBrowsers = parsedMsg.message;
        break;
      case "browserProfiles":
        profilesByBrowser = {
          ...profilesByBrowser,
          [parsedMsg.message.browser]: parsedMsg.message.profiles,
        };
        break;
      case "saveRulesError":
        toast.show(
          "Failed to save rules",
          "error",
          parsedMsg.message?.error ?? "Unknown error"
        );
        break;
      default: {
        const newMessage = parsedMsg.message
          ? JSON.parse(parsedMsg.message)
          : parsedMsg;
        messageBuffer = [...messageBuffer, newMessage];
      }
    }
  }

  function clearAllLogs() {
    messageBuffer = [];
    window.finicky?.sendMessage({ type: "clearLogs" });
  }

  function getSessionToken(): string {
    if (window.__BROWSIFY_TOKEN__) return window.__BROWSIFY_TOKEN__;
    const params = new URLSearchParams(window.location.search);
    return params.get("token") ?? "";
  }

  const _preloadQueue: any[] = window.finicky?._queue ?? [];

  window.finicky = {
    sendMessage: (msg: any) => {
      if (window.webkit?.messageHandlers?.finicky) {
        window.webkit.messageHandlers.finicky.postMessage(JSON.stringify(msg));
      } else {
        const token = getSessionToken();
        fetch(`/api/rpc?token=${encodeURIComponent(token)}`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Browsify-Token": token,
          },
          body: JSON.stringify(msg),
        })
          .then((res) => res.json())
          .then((replies) => {
            if (Array.isArray(replies)) {
              for (const r of replies) handleMessage(r);
            } else if (replies) {
              handleMessage(replies);
            }
          })
          .catch((err) => {
            console.error("Browsify RPC error:", err);
            if (msg.type === "testUrl") {
              testUrlResult.set({ error: "Could not reach Browsify. Check the app is running, then try again." });
            }
          });
      }
    },
    receiveMessage: handleMessage,
  };

  for (const msg of _preloadQueue) {
    handleMessage(msg);
  }

  if (!window.webkit?.messageHandlers?.finicky) {
    window.finicky.sendMessage({ type: "getInitState" });
    const token = getSessionToken();
    if (token && typeof EventSource !== "undefined") {
      const es = new EventSource(
        `/api/events?token=${encodeURIComponent(token)}`
      );
      es.onmessage = (ev) => {
        try {
          handleMessage(JSON.parse(ev.data));
        } catch (err) {
          console.error("Browsify SSE parse error:", err);
        }
      };
    }
  }
  window.finicky.sendMessage({ type: "getRules" });
  window.finicky.sendMessage({ type: "getInstalledBrowsers" });
</script>

<a class="skip-link" href="#main-content">Skip to content</a>
<div class="app-shell">
  <Router>
    <main class={`app-frame ${attrs(materials.frame).class ?? ""}`} style={attrs(materials.frame).style} aria-label="Browsify workspace">
      <div class="layout">
        <TabBar {numErrors} onNavigate={() => scrollArea?.scrollTo(0, 0)} />
        <div class="workspace">
          <div class="container" id="main-content" tabindex="-1" bind:this={scrollArea}>
            <div class="content">
              <Route path="/">
                <StartPage
                  {hasConfig}
                  {updateInfo}
                  {config}
                  {numErrors}
                  {rulesFile}
                  {installedBrowsers}
                />
              </Route>

              <Route path="/preferences">
                <Preferences
                  {hasConfig}
                  {config}
                  {rulesFile}
                  {installedBrowsers}
                  {profilesByBrowser}
                  hasPrimaryConfig={config.hasPrimaryConfig ?? false}
                />
              </Route>

              <Route path="/troubleshoot">
                <LogViewer {messageBuffer} onClearLogs={clearAllLogs} />
              </Route>

              <Route path="/test">
                <TestUrl />
              </Route>

              <Route path="/about">
                <About {version} />
              </Route>

              <Route path="/rules">
                <Rules
                  {rulesFile}
                  {installedBrowsers}
                  {profilesByBrowser}
                  hasPrimaryConfig={config.hasPrimaryConfig ?? false}
                />
              </Route>
            </div>
          </div>
          <div class="footer">
            <span class="connection" class:connected={hasConfig} aria-label={hasConfig ? "Connected to Browsify" : "Connecting to Browsify"}></span>
            {#if config.configPath || rulesFile.path}
              <span class="config-label">Config</span>
              <button class="config-summary" type="button" aria-label={`View ${configPaths.length} configuration ${configPaths.length === 1 ? "file" : "files"}`} on:click={() => toast.show("Configuration files", "info", configPaths.join("\n"), 5000)}>{configPaths.length} config {configPaths.length === 1 ? "file" : "files"}</button>
              {#if config.configPath}
                <button
                  class="config-badge"
                  on:click={() =>
                    showPathToast(
                      basename(config.configPath),
                      "Configuration loaded from",
                      config.configPath
                    )}
                >
                  {basename(config.configPath)}
                </button>
              {/if}
              {#if rulesFile.path && rulesFile.path !== config.configPath}
                <button
                  class="config-badge"
                  on:click={() =>
                    showPathToast(
                      basename(rulesFile.path!),
                      "Configuration loaded from",
                      rulesFile.path!
                    )}
                >
                  {basename(rulesFile.path)}
                </button>
              {/if}
            {:else}
              <span class="config-status">{hasConfig ? "Using default settings" : "Connecting to Browsify…"}</span>
              {#if hasConfig}<Link to="/preferences" class="config-link">Open settings</Link>{/if}
            {/if}
            <span class="spacer"></span>
            <AppearanceMenu />
            {#if config.format}
              <span class="version"
                >{config.pureNativeFastPath
                  ? "Native Fast Path"
                  : "Hybrid QuickJS"}</span
              >
            {/if}
            {#if updateInfo && updateInfo.updateCheckEnabled && !updateInfo.hasUpdate}
              <span class="up-to-date">Up to date</span>
            {/if}
            <span class="version">{version}</span>
          </div>
        </div>
      </div>
      <ToastContainer />
    </main>
  </Router>
</div>

<style>
  .app-shell {
    min-height: 100svh;
    display: grid;
    place-items: center;
    padding: 16px;
  }

  .app-frame {
    position: relative;
    width: 1080px;
    height: 760px;
    max-width: calc(100vw - 32px);
    max-height: calc(100svh - 32px);
    padding: 16px;
    background: var(--browsify-canvas);
    border: 2px solid var(--browsify-border);
    box-shadow: 0 24px 80px rgba(25, 27, 21, 0.1);
  }

  .layout {
    display: flex;
    flex: 1 1 auto;
    min-height: 0;
    align-items: stretch;
    gap: 28px;
    height: 100%;
  }

  .workspace {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding-right: 12px;
  }

  .container {
    padding: 20px 4px 20px 0;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--browsify-border-strong) transparent;
  }

  .footer {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 8px 0 0;
    border-top: 1px solid var(--browsify-border-strong);
    flex-wrap: wrap;
  }

  .version {
    color: var(--text-secondary);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }

  .content { flex: 1; min-width: 0; }

  .connection {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-secondary);
    flex-shrink: 0;
  }

  .connection.connected { background: var(--log-success); }

  .spacer {
    flex: 1;
  }

  .up-to-date {
    color: var(--log-success);
    font-size: 13px;
  }

  .config-label { color: var(--text-secondary); font-size: 13px; }

  .config-summary { display: none; }

  .config-badge {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 13px;
    padding: 6px 10px;
    border-radius: 999px;
    background: var(--browsify-inset);
    border: 1px solid var(--browsify-border);
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
    min-width: 0;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .config-badge:hover {
    background: var(--button-hover);
    color: var(--text-primary);
  }

  .config-status { font-size: 13px; color: var(--text-secondary); }

  .config-link {
    color: var(--accent-color);
    font-size: 13px;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .config-link:hover {
    text-decoration: underline;
  }

  @media (max-width: 700px) {
    .app-shell { padding: 8px; }
    .app-frame {
      max-width: calc(100vw - 16px);
      max-height: calc(100svh - 16px);
      padding: 8px;
      border-radius: 30px;
    }
    .layout { gap: 16px; }
    .workspace { padding-right: 4px; }
    .container { padding-top: 16px; }
    .footer { gap: 6px; padding: 8px 0; }
    .footer .version, .up-to-date, .config-label { display: none; }
    .config-badge { display: none; }
    .config-summary { display: inline-flex; align-items: center; min-height: 44px; border: 0; border-radius: 999px; padding: 8px 12px; background: var(--browsify-inset); color: var(--text-secondary); font-size: 13px; cursor: pointer; }
    .config-summary:hover { color: var(--text-primary); }
    .spacer { display: none; }
  }
</style>
