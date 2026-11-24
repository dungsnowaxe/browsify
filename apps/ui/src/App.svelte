<script lang="ts">
  import { attrs, materials } from "./design-system/styles";
  import { Router, Route } from "svelte-routing";
  import { onDestroy, onMount } from "svelte";
  import LogViewer from "./pages/LogViewer.svelte";
  import StartPage from "./pages/StartPage.svelte";
  import Preferences from "./pages/Preferences.svelte";
  import TabBar from "./components/TabBar.svelte";
  import About from "./pages/About.svelte";
  import TestUrl from "./pages/TestUrl.svelte";
  import Rules from "./pages/Rules.svelte";
  import ToastContainer from "./components/ToastContainer.svelte";
  import AppearanceMenu from "./components/AppearanceMenu.svelte";
  import type { LogEntry, UpdateInfo, ConfigInfo, Choice, ProfileChoices, LoginItemStatus } from "./types";
  import { receiveUrlTestResult } from "./lib/testUrlStore";
  import { toast } from "./lib/toast";

  import {draft, draftError, saving, save, discard, startDraft} from "./lib/configDraft";
  import {equal} from "./lib/configMerge";
  import ImportConfig from "./components/ImportConfig.svelte";
  onMount(startDraft);
  $: dirty = $draft && !equal($draft.original,$draft.current);
  let version = "v0.0.0";
  let updateChecking = false;

  let scrollArea: HTMLDivElement;
  let hasConfig = false;
  let config: ConfigInfo = { configPath: "" };
  let messageBuffer: LogEntry[] = [];
  let updateInfo: UpdateInfo | null = null;
  let installedBrowsers: string[] = [];
  let profilesByBrowser: Record<string, string[]> = {};

  let browserChoices: Choice[] | null = null;
  let profileChoices: Record<string, ProfileChoices> = {};
  let loginItem: LoginItemStatus = "unavailable";
  let loginSaving = false;

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
        break;
      case "updateCheckError":
        updateChecking = false;
        toast.show("Could not check for updates", "error", parsedMsg.message?.error);
        break;
      case "config":
        hasConfig = true;
        if (parsedMsg.message) {
          config = parsedMsg.message;
        }
        break;
      case "updateInfo":
        updateInfo = parsedMsg.message;
        if (window.webkit?.messageHandlers?.finicky) updateChecking = false;
        break;
      case "updateAction":
        toast.show("Browsify updates", "info", parsedMsg.message);
        break;
      case "testUrlResult":
        receiveUrlTestResult(parsedMsg.message, parsedMsg.requestId);
        break;
      case "ruleEditorError":
        draftError.set(parsedMsg.message.error);
        break;
      case "browserChoices":
        browserChoices = parsedMsg.message;
        break;
      case "profileChoices":
        profileChoices = { ...profileChoices, [parsedMsg.message.browser]: parsedMsg.message };
        break;
      case "loginItem":
        loginSaving = false;
        loginItem = parsedMsg.message.status ?? "unavailable";
        if (parsedMsg.message.error) toast.show("Could not change Open at Login", "error", parsedMsg.message.error);
        break;
      case "installedBrowsers":
        installedBrowsers = parsedMsg.message;
        break;
      case "browserProfilesError":
        toast.show("Could not load profiles", "error", parsedMsg.message.error);
        break;
      case "browserProfiles":
        profilesByBrowser = {
          ...profilesByBrowser,
          [parsedMsg.message.browser]: parsedMsg.message.profiles,
        };
        break;
      case "saveRulesError":
        toast.show(
          "Could not update rules",
          "error",
          "Could not apply your rules. Check them and try again."
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
      if (msg.type === "setLoginItem") loginSaving = true;
      if (msg.type === "checkForUpdates") { updateChecking = true; updateInfo = null; }
      if (window.webkit?.messageHandlers?.finicky) {
        window.webkit.messageHandlers.finicky.postMessage(JSON.stringify(msg));
      } else {
        const token = getSessionToken();
        void fetch(`/api/rpc?token=${encodeURIComponent(token)}`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Browsify-Token": token,
          },
          body: JSON.stringify(msg),
        })
          .then((res) => {
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            return res.json();
          })
          .then((replies) => {
            const messages = Array.isArray(replies) ? replies : [replies];
            // Signed updates acknowledge immediately; progress/results arrive over SSE.
            if (msg.type === "checkForUpdates" && !messages.some((reply) => reply?.type === "updateAction" || reply?.type === "updateInfo" || reply?.type === "updateCheckError")) {
              throw new Error("Browsify did not return an update result");
            }
            for (const reply of messages) if (reply) handleMessage(reply);
          })
          .catch((err) => {
            console.error("Browsify RPC error:", err);
            if (msg.type === "testUrl") {
              receiveUrlTestResult({ error: "Could not reach Browsify. Check the app is running, then try again." }, msg.requestId);
            } else if (msg.type === "editAdvancedRules") {
              draftError.set("Could not open the config file.");
            } else if (msg.type === "getBrowserChoices") {
              browserChoices = [];
              draftError.set("Could not load installed browsers. Try refreshing the list.");
            } else if (msg.type === "getProfileChoices") {
              profileChoices = { ...profileChoices, [msg.browser]: { profiles: [], error: true } };
            } else if (msg.type === "setLoginItem") {
              loginSaving = false;
              toast.show("Could not change Open at Login", "error", "Check that Browsify is running and try again.");
            } else if (msg.type === "checkForUpdates") {
              updateChecking = false;
              toast.show("Could not check for updates", "error", "Check that Browsify is running, then try again.");
            }
          }).finally(() => {
            if (msg.type === "checkForUpdates") updateChecking = false;
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
      onDestroy(() => es.close());
      es.onmessage = (ev) => {
        try {
          handleMessage(JSON.parse(ev.data));
        } catch (err) {
          console.error("Browsify SSE parse error:", err);
        }
      };
    }
  }
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
              <ImportConfig />
              <Route path="/">
                <StartPage
                  {hasConfig}
                  {updateInfo}
                  {config}
                  {numErrors}
                  {installedBrowsers}
                />
              </Route>

              <Route path="/preferences">
                <Preferences
                  {hasConfig}
                  {config}
                  {installedBrowsers}
                  {profilesByBrowser}
                  {loginItem}
                  {loginSaving}
                  {updateInfo}
                  {updateChecking}
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
                  {browserChoices}
                  {profileChoices}
                />
              </Route>
            </div>
          </div>
          {#if $draftError}<p role="alert">{$draftError}</p>{/if}
          {#if dirty}<div class="footer" aria-label="Unsaved configuration">
            <span>Unsaved changes</span><span class="spacer"></span>
            <button class="app-button" disabled={$saving} on:click={discard}>Discard changes</button>
            <button class="app-button" disabled={$saving} on:click={save}>{$saving ? "Saving…" : "Save changes"}</button>
          </div>{/if}
          <div class="footer">
            <span class="connection" class:connected={hasConfig} aria-label={hasConfig ? "Connected to Browsify" : "Connecting to Browsify"}></span>
            <span class="spacer"></span>
            <AppearanceMenu />
            {#if updateInfo?.status === "upToDate"}
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
    .footer .version, .up-to-date { display: none; }
  }
</style>
