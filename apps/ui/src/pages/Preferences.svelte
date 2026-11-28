<script lang="ts">
  import { onMount } from "svelte";
  import PageContainer from "../components/PageContainer.svelte";
  import BrowserTargetPicker from "../components/BrowserTargetPicker.svelte";
  import OptionRow from "../components/OptionRow.svelte";
  import ImportConfig from "../components/ImportConfig.svelte";
  import AppearanceMenu from "../components/AppearanceMenu.svelte";
  import { toast } from "../lib/toast";
  import { draft, edit, saving } from "../lib/configDraft";
  import type {
    Choice,
    ConfigInfo,
    ConfigOptions,
    LoginItemStatus,
    ProfileChoices,
    UpdateInfo,
  } from "../types";

  export let hasConfig: boolean;
  export let connectionStatus: "connecting" | "connected" | "disconnected" = "connecting";
  export let config: ConfigInfo;
  export let installedBrowsers: string[] = [];
  export let profilesByBrowser: Record<string, string[]> = {};
  export let browserChoices: Choice[] | null = null;
  export let profileChoices: Record<string, ProfileChoices> = {};
  export let updateInfo: UpdateInfo | null = null;
  export let updateChecking = false;
  export let loginItem: LoginItemStatus = "unavailable";
  export let loginSaving = false;
  export let onRetry: () => void = () => {};

  onMount(() => {
    const refreshLogin = () => window.finicky.sendMessage({ type: "getLoginItem" });
    refreshLogin();
    window.finicky.sendMessage({ type: "getBrowserChoices" });
    window.addEventListener("focus", refreshLogin);
    return () => window.removeEventListener("focus", refreshLogin);
  });

  let keepRunning = true;
  let hideIcon = false;
  let logRequests = false;
  let checkForUpdates = true;

  $: prefs = $draft?.current.settings ?? {};
  $: origPrefs = $draft?.original.settings ?? {};
  $: keepRunning = prefs.options?.keepRunning ?? config.options?.keepRunning ?? true;
  $: hideIcon = prefs.options?.hideIcon ?? config.options?.hideIcon ?? false;
  $: logRequests = prefs.options?.logRequests ?? config.options?.logRequests ?? false;
  $: checkForUpdates = prefs.options?.checkForUpdates ?? config.options?.checkForUpdates ?? true;
  $: defaultBrowser = prefs.defaultBrowser ?? "";
  $: defaultProfile = prefs.defaultProfile ?? "";
  $: defaultTargetDirty =
    !!$draft &&
    ((prefs.defaultBrowser ?? "") !== (origPrefs.defaultBrowser ?? "") ||
      (prefs.defaultProfile ?? "") !== (origPrefs.defaultProfile ?? ""));
  $: keepRunningDirty = !!$draft && prefs.options?.keepRunning !== origPrefs.options?.keepRunning;
  $: hideIconDirty = !!$draft && prefs.options?.hideIcon !== origPrefs.options?.hideIcon;
  $: logRequestsDirty = !!$draft && prefs.options?.logRequests !== origPrefs.options?.logRequests;
  $: checkForUpdatesDirty =
    !!$draft && prefs.options?.checkForUpdates !== origPrefs.options?.checkForUpdates;
  $: hasBehaviorOverrides = Object.values(prefs.options ?? {}).some((v) => v !== undefined);
  $: draftLockedText = $saving
    ? "Saving changes…"
    : "Open Browsify to connect and change this setting.";

  function onLockedClick() {
    if ($saving) return;
    toast.show("Waiting for Browsify", "info", "Open Browsify to connect and change settings.");
  }

  const DEFAULT_OPTIONS: Record<keyof ConfigOptions, boolean> = {
    keepRunning: true,
    hideIcon: false,
    logRequests: false,
    checkForUpdates: true,
  };

  function setOption(key: keyof ConfigOptions, value: boolean) {
    const origVal = $draft?.original.settings.options?.[key];
    const fallback = config.options?.[key] ?? DEFAULT_OPTIONS[key];
    edit((v) => {
      const next = { ...v.settings.options };
      if (origVal === undefined && value === fallback) {
        delete next[key];
      } else {
        next[key] = value;
      }
      v.settings.options = next;
    });
  }

  function setDefaultTarget(browser: string, profile: string) {
    edit((v) => {
      v.settings.defaultBrowser = browser || undefined;
      v.settings.defaultProfile = profile || undefined;
    });
  }

  function resetBehaviorOptions() {
    edit((v) => {
      v.settings.options = {};
    });
    toast.info("Behavior options reset to defaults — Save changes to apply", 3500);
  }

  function requestProfiles(browser: string) {
    window.finicky.sendMessage({ type: "getProfileChoices", browser });
    window.finicky.sendMessage({ type: "getBrowserProfiles", browser });
  }

  $: updateStatusText =
    updateInfo?.error ??
    (updateInfo?.status === "checking" || updateChecking
      ? "Checking for updates…"
      : updateInfo?.status === "downloading"
        ? "Downloading and verifying update…"
        : updateInfo?.status === "ready"
          ? `${updateInfo.version} is ready to install. Restart Browsify to apply it.`
          : updateInfo?.status === "skipped"
            ? `${updateInfo.version} is skipped. You can still download it manually below.`
            : updateInfo?.status === "deferred"
              ? `${updateInfo.version} reminder postponed for 24 hours.`
              : updateInfo?.status === "upToDate"
                ? "You have the latest stable release."
                : updateInfo?.hasUpdate
                  ? `${updateInfo.version} is available.`
                  : "Manual checks are always available, even with Automatic Updates off.");
</script>

<PageContainer title="Preferences">
  {#snippet description()}
    Choose where unmatched links open and how Browsify runs on your Mac.
  {/snippet}

  {#if !hasConfig}
    <section class="section-card glass-panel disconnected-card" aria-labelledby="prefs-connection-heading">
      <h2 id="prefs-connection-heading">
        {connectionStatus === "disconnected" ? "Browsify is not connected" : "Connecting to Browsify…"}
      </h2>
      <p>
        {#if connectionStatus === "disconnected"}
          Open Browsify from Applications, Spotlight, or run <code>browsify ui</code>, then retry to view or change your Primary Config, macOS integration, updates, or config imports.
        {:else}
          Loading your active configuration and macOS settings…
        {/if}
      </p>
      {#if connectionStatus === "disconnected"}
        <button type="button" class="app-button primary" on:click={onRetry}>
          Retry connection
        </button>
      {/if}
    </section>

    <section class="section-card glass-panel" aria-labelledby="appearance-heading">
      <div class="section-header">
        <div>
          <h2 id="appearance-heading">Appearance</h2>
          <p>Choose how this window looks. Applies immediately.</p>
        </div>
        <AppearanceMenu />
      </div>
    </section>
  {:else}
    <div class="pref-group" role="region" aria-labelledby="group-config-heading">
      <div class="group-header">
        <h2 id="group-config-heading" class="group-title">Primary Config</h2>
        <p>Staged in your Config Draft and saved with your rules when you click Save changes.</p>
      </div>

      <section class="section-card glass-panel" aria-labelledby="browser-heading">
        <div class="section-header">
          <div>
            <h3 id="browser-heading" class="card-title heading-with-badge">
              Default Browser Target
              {#if defaultTargetDirty}
                <span class="unsaved-pill">Unsaved</span>
              {/if}
            </h3>
            <p>Used when no Handler Rule matches a link.</p>
          </div>
        </div>
        <BrowserTargetPicker
          browser={defaultBrowser}
          profile={defaultProfile}
          {browserChoices}
          {profileChoices}
          {installedBrowsers}
          {profilesByBrowser}
          disabled={!$draft || $saving}
          allowCustom={true}
          browserPlaceholder="Use built-in default"
          onRequestProfiles={requestProfiles}
          onBrowserChange={(browser, profile) => setDefaultTarget(browser, profile)}
          onProfileChange={(profile) => setDefaultTarget(defaultBrowser, profile)}
        />
      </section>

      <section class="behavior" aria-labelledby="behavior-heading">
        <div class="section-header">
          <div>
            <h3 id="behavior-heading" class="card-title">App behavior</h3>
          </div>
        </div>
        <div class="options-grid">
          <OptionRow
            label="Keep running"
            hint="Stay available in the background"
            modified={keepRunningDirty}
            bind:checked={keepRunning}
            locked={!$draft || $saving}
            lockedText={draftLockedText}
            {onLockedClick}
            onchange={() => setOption("keepRunning", keepRunning)}
          />
          <OptionRow
            label="Hide menu bar icon"
            hint="Keep Browsify out of the menu bar. Reopen anytime from Applications, Spotlight, or browsify ui."
            modified={hideIconDirty}
            bind:checked={hideIcon}
            locked={!$draft || $saving}
            lockedText={draftLockedText}
            {onLockedClick}
            onchange={() => setOption("hideIcon", hideIcon)}
          />
          <OptionRow
            label="Log URL requests"
            hint="Write URL handling activity to Activity logs"
            modified={logRequestsDirty}
            bind:checked={logRequests}
            locked={!$draft || $saving}
            lockedText={draftLockedText}
            {onLockedClick}
            onchange={() => setOption("logRequests", logRequests)}
          />
          <OptionRow
            label="Automatic Updates"
            hint="Check daily and download signed updates. Ask before restarting."
            modified={checkForUpdatesDirty}
            bind:checked={checkForUpdates}
            locked={!$draft || $saving}
            lockedText={draftLockedText}
            {onLockedClick}
            onchange={() => setOption("checkForUpdates", checkForUpdates)}
          />
        </div>
        {#if hasBehaviorOverrides}
          <div class="settings-actions">
            <button
              class="app-button"
              type="button"
              disabled={!$draft || $saving}
              on:click={resetBehaviorOptions}
            >
              Reset behavior options
            </button>
          </div>
        {/if}
      </section>
    </div>

    <div class="pref-group" role="region" aria-labelledby="group-local-heading">
      <div class="group-header">
        <h2 id="group-local-heading" class="group-title">This Mac & app</h2>
        <p>Applies immediately — no save required.</p>
      </div>

      <section class="section-card glass-panel" aria-labelledby="appearance-heading">
        <div class="section-header">
          <div>
            <h3 id="appearance-heading" class="card-title">Appearance</h3>
            <p>Choose how this window looks.</p>
          </div>
          <AppearanceMenu />
        </div>
      </section>

      <section class="section-card glass-panel" aria-labelledby="macos-heading">
        <div class="section-header">
          <div>
            <h3 id="macos-heading" class="card-title">macOS integration</h3>
          </div>
        </div>

        <div class="handler-row">
          <div>
            <h4>Default link handler</h4>
            <p>
              {config.isDefaultBrowser
                ? "Browsify handles links opened from other apps."
                : "Make Browsify your Mac’s default browser to route links."}
            </p>
          </div>
          {#if config.isDefaultBrowser}
            <span class="ready-label">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <circle cx="12" cy="12" r="10" />
                <path d="m9 12 2 2 4-4" />
              </svg>
              Active
            </span>
          {:else}
            <button
              class="app-button primary"
              type="button"
              on:click={() => window.finicky.sendMessage({ type: "setDefaultBrowser" })}
            >
              Set as default
            </button>
          {/if}
        </div>

        {#if loginItem !== "unavailable"}
          <OptionRow
            flat={true}
            label="Open at Login"
            hint={loginItem === "needsApproval"
              ? "Allow Browsify in System Settings → General → Login Items."
              : "Start Browsify when you sign in to your Mac."}
            hintTone={loginItem === "needsApproval" ? "warning" : "default"}
            checked={loginItem !== "off"}
            locked={loginSaving}
            lockedText="Updating Open at Login…"
            onchange={() =>
              window.finicky.sendMessage({ type: "setLoginItem", enabled: loginItem === "off" })}
          />
        {/if}
      </section>

      <section class="section-card glass-panel" aria-labelledby="updates-heading">
        <div class="section-header">
          <div>
            <h3 id="updates-heading" class="card-title">Browsify updates</h3>
            <p aria-live="polite">{updateStatusText}</p>
          </div>
          <button
            class="app-button"
            type="button"
            disabled={updateChecking || updateInfo?.status === "checking" || updateInfo?.status === "downloading"}
            on:click={() => window.finicky.sendMessage({ type: "checkForUpdates" })}
          >
            Check for updates…
          </button>
        </div>

        {#if updateInfo?.status === "ready"}
          <div class="update-actions">
            <button
              class="app-button primary"
              type="button"
              on:click={() => window.finicky.sendMessage({ type: "restartToUpdate" })}
            >
              Restart to Update
            </button>
            <button
              class="app-button"
              type="button"
              on:click={() => window.finicky.sendMessage({ type: "deferUpdate" })}
            >
              Remind me in 24 hours
            </button>
            <button
              class="app-button"
              type="button"
              on:click={() => window.finicky.sendMessage({ type: "skipUpdate" })}
            >
              Skip this release
            </button>
          </div>
        {:else if updateInfo?.hasUpdate && (updateInfo.status === "available" || updateInfo.status === "skipped" || updateInfo.status === "deferred" || updateInfo.status === "error")}
          <div class="update-actions">
            <button
              class="app-button primary"
              type="button"
              on:click={() => window.finicky.sendMessage({ type: "downloadUpdate" })}
            >
              Download update
            </button>
            {#if updateInfo.status === "available"}
              <button
                class="app-button"
                type="button"
                on:click={() => window.finicky.sendMessage({ type: "deferUpdate" })}
              >
                Remind me in 24 hours
              </button>
              <button
                class="app-button"
                type="button"
                on:click={() => window.finicky.sendMessage({ type: "skipUpdate" })}
              >
                Skip this release
              </button>
            {/if}
          </div>
        {/if}

        <a
          class="release-link"
          href={updateInfo?.releaseUrl || "https://github.com/snowaxe/browsify/releases"}
          target="_blank"
          rel="noopener noreferrer"
        >
          Release notes and manual download ↗
        </a>
      </section>

      <ImportConfig mode="settings" connected={hasConfig} {browserChoices} {installedBrowsers} />
    </div>
  {/if}
</PageContainer>

<style>
  .pref-group {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .pref-group + .pref-group {
    margin-top: 8px;
    padding-top: 20px;
    border-top: 1px solid var(--browsify-border-strong);
  }

  .group-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .group-title {
    margin: 0;
    font: 500 18px/1.25 var(--font-display);
    letter-spacing: -0.025em;
    color: var(--text-primary);
  }

  .section-card {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .disconnected-card {
    align-items: flex-start;
    gap: 12px;
  }

  .disconnected-card code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    background: var(--browsify-inset);
    padding: 1px 5px;
    border-radius: 4px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .handler-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding-top: 14px;
    border-top: 1px solid var(--browsify-border-strong);
  }

  h2,
  .card-title {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .heading-with-badge {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .unsaved-pill {
    font-size: 12px;
    font-weight: 500;
    color: var(--log-warning);
    padding: 0 7px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--log-warning) 55%, var(--browsify-control-border));
    background: var(--browsify-inset);
  }

  h4 {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  p {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .handler-row button {
    flex-shrink: 0;
  }

  .ready-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--log-success);
    white-space: nowrap;
  }

  .settings-actions,
  .update-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .release-link {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    min-height: 44px;
    font-size: 14px;
  }

  .behavior {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .options-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  @media (max-width: 800px) {
    .handler-row,
    .section-header {
      align-items: flex-start;
      flex-direction: column;
      gap: 12px;
    }

    .options-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
