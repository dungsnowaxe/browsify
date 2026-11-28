<script lang="ts">
  import { attrs, materials } from "./design-system/styles";
  import { Router, Route, Link } from "svelte-routing";
  import { onDestroy, onMount } from "svelte";
  import LogViewer from "./pages/LogViewer.svelte";
  import StartPage from "./pages/StartPage.svelte";
  import Preferences from "./pages/Preferences.svelte";
  import TabBar from "./components/TabBar.svelte";
  import About from "./pages/About.svelte";
  import TestUrl from "./pages/TestUrl.svelte";
  import Rules from "./pages/Rules.svelte";
  import ToastContainer from "./components/ToastContainer.svelte";
  import DraftDialog from "./components/DraftDialog.svelte";
  import AppFooter from "./components/AppFooter.svelte";
  import type { LogEntry, UpdateInfo, ConfigInfo, Choice, ProfileChoices, LoginItemStatus } from "./types";
  import { activeDialog, askConfirmClearLogs } from "./lib/draftDialog";
  import { receiveUrlTestResult } from "./lib/testUrlStore";
  import { toast } from "./lib/toast";
  import {
    draft,
    draftError,
    clearDraftError,
    countInvalidRules,
    requestFocusInvalidRule,
    saving,
    save,
    discard,
    refresh,
    startDraft,
  } from "./lib/configDraft";
  import { equal } from "./lib/configMerge";

  let manualRetrying = false;
  let DevTargetBarComponent: typeof import("./components/DevTargetBar.svelte").default | null = null;

  if (import.meta.env.DEV) {
    void import("./components/DevTargetBar.svelte").then((mod) => {
      DevTargetBarComponent = mod.default;
    });
  }

  onMount(() => {
    const stopDraft = startDraft();
    const onKeydown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
        if (dirty && !$saving && !$activeDialog) {
          event.preventDefault();
          if (invalidRuleCount > 0) {
            requestFocusInvalidRule();
          }
          void save();
        }
      }
    };
    window.addEventListener("keydown", onKeydown);
    return () => {
      stopDraft();
      window.removeEventListener("keydown", onKeydown);
    };
  });

  $: dirty = $draft && !equal($draft.original, $draft.current);
  $: invalidRuleCount = $draft ? countInvalidRules($draft.current.rows) : 0;

  let version = "";
  let updateChecking = false;
  let connectionStatus: "connecting" | "connected" | "disconnected" = "connecting";

  let scrollArea: HTMLElement;
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

  $: showDraftError = !!$draftError && connectionStatus !== "disconnected";
  $: toastOffsetLevel = (dirty && showDraftError ? 2 : dirty || showDraftError ? 1 : 0) as 0 | 1 | 2;

  function handleMessage(msg: any) {
    const parsedMsg = typeof msg === "string" ? JSON.parse(msg) : msg;
    switch (parsedMsg.type) {
      case "version":
        version = parsedMsg.message;
        connectionStatus = "connected";
        manualRetrying = false;
        break;
      case "buildInfo":
        break;
      case "updateCheckError":
        updateChecking = false;
        toast.show("Could not check for updates", "error", parsedMsg.message?.error);
        break;
      case "config":
        hasConfig = true;
        connectionStatus = "connected";
        manualRetrying = false;
        if (parsedMsg.message) {
          config = parsedMsg.message;
        }
        break;
      case "updateInfo":
        updateInfo = parsedMsg.message;
        if (parsedMsg.message?.status !== "checking") {
          updateChecking = false;
        }
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

  async function clearAllLogs() {
    if (!messageBuffer.length) return;
    const confirmed = await askConfirmClearLogs(messageBuffer.length);
    if (!confirmed) return;
    messageBuffer = [];
    window.finicky?.sendMessage({ type: "clearLogs" });
    toast.info("Activity logs cleared", 2600);
  }

  function getSessionToken(): string {
    if (window.__BROWSIFY_TOKEN__) return window.__BROWSIFY_TOKEN__;
    const params = new URLSearchParams(window.location.search);
    return params.get("token") ?? "";
  }

  function retryConnection() {
    manualRetrying = true;
    connectionStatus = "connecting";
    clearDraftError();
    window.finicky.sendMessage({ type: "getInitState" });
    window.finicky.sendMessage({ type: "getInstalledBrowsers" });
    window.finicky.sendMessage({ type: "getBrowserChoices" });
    void refresh();
  }

  const _preloadQueue: any[] = window.finicky?._queue ?? [];

  window.finicky = {
    sendMessage: (msg: any) => {
      if (msg.type === "setLoginItem") loginSaving = true;
      if (msg.type === "checkForUpdates") {
        updateChecking = true;
        updateInfo = null;
      }
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
            if (
              msg.type === "checkForUpdates" &&
              !messages.some(
                (reply) =>
                  reply?.type === "updateAction" ||
                  reply?.type === "updateInfo" ||
                  reply?.type === "updateCheckError"
              )
            ) {
              throw new Error("Browsify did not return an update result");
            }
            for (const reply of messages) if (reply) handleMessage(reply);
          })
          .catch((err) => {
            console.error("Browsify RPC error:", err);
            if (msg.type === "getInitState") {
              connectionStatus = "disconnected";
              if (manualRetrying) {
                manualRetrying = false;
                toast.error(
                  "Browsify isn’t running",
                  "Open Browsify from Applications, Spotlight, or run browsify ui, then try again."
                );
              }
            } else if (msg.type === "testUrl") {
              receiveUrlTestResult(
                { error: "Could not reach Browsify. Check the app is running, then try again." },
                msg.requestId
              );
            } else if (msg.type === "editAdvancedRules") {
              draftError.set("Could not open your Primary Config.");
            } else if (msg.type === "getBrowserChoices") {
              browserChoices = [];
              if (hasConfig) {
                draftError.set("Could not load installed browsers. Try refreshing the list.");
              }
            } else if (msg.type === "getProfileChoices") {
              profileChoices = { ...profileChoices, [msg.browser]: { profiles: [], error: true } };
            } else if (msg.type === "setLoginItem") {
              loginSaving = false;
              toast.show("Could not change Open at Login", "error", "Check that Browsify is running and try again.");
            } else if (msg.type === "checkForUpdates") {
              updateChecking = false;
              toast.show("Could not check for updates", "error", "Check that Browsify is running, then try again.");
            } else if (msg.type === "skipUpdate" || msg.type === "deferUpdate") {
              toast.show("Could not update release preference", "error", "Check that Browsify is running, then try again.");
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
      onDestroy(() => es.close());
      es.onmessage = (ev) => {
        try {
          handleMessage(JSON.parse(ev.data));
        } catch (err) {
          console.error("Browsify SSE parse error:", err);
        }
      };
      es.onerror = () => {
        if (!hasConfig) connectionStatus = "disconnected";
      };
    }
  }
  window.finicky.sendMessage({ type: "getInstalledBrowsers" });
  window.finicky.sendMessage({ type: "getBrowserChoices" });
</script>

<a class="skip-link" href="#main-content">Skip to content</a>
{#if import.meta.env.DEV && DevTargetBarComponent}
  <svelte:component
    this={DevTargetBarComponent}
    {connectionStatus}
    configPath={config.configPath}
    dirty={!!dirty}
  />
{/if}
<div class="app-shell">
  <Router>
    <div
      class={`app-frame ${attrs(materials.frame).class ?? ""}`}
      style={attrs(materials.frame).style}
    >
      <div class="layout">
        <TabBar
          {numErrors}
          onNavigate={() => {
            scrollArea?.scrollTo(0, 0);
          }}
        />
        <div class="workspace">
          <main class="container" id="main-content" tabindex="-1" bind:this={scrollArea}>
            <div class="content">
              <Route path="/">
                <StartPage
                  {hasConfig}
                  {connectionStatus}
                  {updateInfo}
                  {config}
                  {numErrors}
                  {installedBrowsers}
                  {browserChoices}
                  onRetry={retryConnection}
                />
              </Route>

              <Route path="/preferences">
                <Preferences
                  {hasConfig}
                  {connectionStatus}
                  {config}
                  {installedBrowsers}
                  {profilesByBrowser}
                  {browserChoices}
                  {profileChoices}
                  {loginItem}
                  {loginSaving}
                  {updateInfo}
                  {updateChecking}
                  onRetry={retryConnection}
                />
              </Route>

              <Route path="/troubleshoot">
                <LogViewer
                  {messageBuffer}
                  {connectionStatus}
                  onClearLogs={clearAllLogs}
                  onRetry={retryConnection}
                />
              </Route>

              <Route path="/test">
                <TestUrl {connectionStatus} onRetry={retryConnection} />
              </Route>

              <Route path="/about">
                <About {version} />
              </Route>

              <Route path="/rules">
                <Rules
                  {browserChoices}
                  {profileChoices}
                  {installedBrowsers}
                  {profilesByBrowser}
                  {connectionStatus}
                  onRetry={retryConnection}
                />
              </Route>
            </div>
          </main>

          {#if showDraftError}
            <div class="draft-error" role="alert">
              <span>{$draftError}</span>
              <div class="draft-error-actions">
                {#if dirty}
                  <button
                    type="button"
                    class="inline-action"
                    disabled={$saving || invalidRuleCount > 0}
                    on:click={save}
                  >
                    Retry save
                  </button>
                {/if}
                <button type="button" class="inline-action" on:click={clearDraftError}>
                  Dismiss
                </button>
              </div>
            </div>
          {/if}

          {#if dirty}
            <div class="footer draft-footer" role="region" aria-label="Unsaved configuration">
              <span class="draft-status">
                {#if invalidRuleCount > 0}
                  <Link to="/rules" class="draft-link" on:click={requestFocusInvalidRule}>
                    {invalidRuleCount} {invalidRuleCount === 1 ? "rule needs" : "rules need"} attention before saving →
                  </Link>
                {:else}
                  Unsaved changes
                {/if}
              </span>
              <span class="spacer"></span>
              <button type="button" class="app-button danger" disabled={$saving} on:click={discard}>
                Discard changes
              </button>
              <button
                type="button"
                class="app-button primary"
                title="Save changes (⌘S)"
                disabled={$saving || invalidRuleCount > 0}
                on:click={save}
              >
                {$saving ? "Saving…" : "Save changes"}
              </button>
            </div>
          {/if}

          <AppFooter
            {connectionStatus}
            {updateInfo}
            {version}
            onRetry={retryConnection}
          />
        </div>
      </div>
      <ToastContainer offsetLevel={toastOffsetLevel} />
      <DraftDialog />
    </div>
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
    background-color: var(--browsify-canvas);
    border: 2px solid var(--browsify-border);
    box-shadow: 0 24px 80px color-mix(in srgb, var(--browsify-rail) 18%, transparent);
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
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding-right: 12px;
  }

  .container {
    padding: 16px 4px 16px 0;
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

  .draft-error {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: space-between;
    gap: 8px 12px;
    flex-wrap: wrap;
    margin: 8px 0 4px;
    padding: 8px 12px;
    border-radius: var(--browsify-radius-control);
    border: 1px solid var(--log-error);
    background: color-mix(in srgb, var(--log-error) 8%, var(--browsify-panel));
    color: var(--log-error);
    font-size: 13px;
  }

  .draft-error-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .inline-action {
    background: none;
    border: none;
    color: inherit;
    font-weight: 500;
    text-decoration: underline;
    cursor: pointer;
    min-height: 44px;
    padding: 0 10px;
    border-radius: 10px;
  }

  .inline-action:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .footer {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 6px 0 0;
    border-top: 1px solid var(--browsify-border-strong);
    flex-wrap: wrap;
  }

  .draft-footer {
    padding-bottom: 6px;
  }

  .draft-status {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .draft-status :global(.draft-link) {
    color: var(--log-warning);
    text-decoration: underline;
    display: inline-flex;
    align-items: center;
    min-height: 44px;
  }

  .content { flex: 1; min-width: 0; display: flex; flex-direction: column; }

  .spacer {
    flex: 1;
  }

  @media (max-width: 700px) {
    .app-shell { padding: 8px; }
    .app-frame {
      width: 100%;
      height: calc(100svh - 16px);
      max-width: calc(100vw - 16px);
      max-height: calc(100svh - 16px);
      padding: 10px;
      border-radius: 28px;
    }
    .layout {
      flex-direction: column;
      gap: 10px;
    }
    .workspace {
      padding-right: 0;
    }
    .container {
      padding: 6px 2px 18px 2px;
    }
    .footer { gap: 6px; padding: 6px 0 0; }
  }
</style>
