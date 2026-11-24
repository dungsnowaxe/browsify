<script lang="ts">
  import { onMount } from "svelte";
  import PageContainer from "../components/PageContainer.svelte";
  import BrowserProfileSelector from "../components/BrowserProfileSelector.svelte";
  import OptionRow from "../components/OptionRow.svelte";
  import { toast } from "../lib/toast";
  import type { ConfigInfo, ConfigOptions, LoginItemStatus, UpdateInfo } from "../types";

  export let hasConfig: boolean;
  export let config: ConfigInfo;
  export let installedBrowsers: string[] = [];
  export let profilesByBrowser: Record<string, string[]> = {};
  import { draft, edit, saving } from "../lib/configDraft";
  export let updateInfo: UpdateInfo | null = null;
  export let updateChecking = false;
  export let loginItem: LoginItemStatus = "unavailable";
  export let loginSaving = false;
  onMount(() => {
    const refresh = () => window.finicky.sendMessage({ type: "getLoginItem" });
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  });

  let keepRunning=true, hideIcon=false, logRequests=false, checkForUpdates=false;
  let defaultBrowser="", defaultProfile="", defaultBrowserIsCustom=false, defaultProfileIsCustom=false;
  $: prefs = $draft?.current.settings ?? {};
  $: keepRunning=prefs.options?.keepRunning ?? true;
  $: hideIcon=prefs.options?.hideIcon ?? false;
  $: logRequests=prefs.options?.logRequests ?? false;
  $: checkForUpdates=prefs.options?.checkForUpdates ?? false;
  $: defaultBrowser=prefs.defaultBrowser ?? "";
  $: defaultProfile=prefs.defaultProfile ?? "";
  function reconcileChoices(browsers: string[], profiles: Record<string, string[]>) {
    if (defaultBrowser) defaultBrowserIsCustom = !browsers.includes(defaultBrowser);
    if (defaultProfile) defaultProfileIsCustom = !(profiles[defaultBrowser] ?? []).includes(defaultProfile);
  }

  $: reconcileChoices(installedBrowsers, profilesByBrowser);
  $: hasOverrides = !!prefs.defaultBrowser || Object.values(prefs.options ?? {}).some(v=>v!==undefined);
  $: if (defaultBrowser && !defaultBrowserIsCustom && profilesByBrowser[defaultBrowser] === undefined) {
    window.finicky.sendMessage({ type: "getBrowserProfiles", browser: defaultBrowser });
  }

  function onLockedClick() {
    toast.show("Waiting for Browsify", "info", "Open Browsify to connect and change settings.");
  }
  function setOption(key:keyof ConfigOptions,value:boolean) {
    edit(v=>{v.settings.options={...v.settings.options,[key]:value};});
  }
  function updateTarget() {
    edit(v=>{v.settings.defaultBrowser=defaultBrowser || undefined;v.settings.defaultProfile=defaultProfile || undefined;});
  }
  function resetSettings() {edit(v=>{v.settings={options:{}}});}
</script>

<PageContainer eyebrow="Preferences" title="Settings">
  {#snippet description()}Choose where unmatched links open and how Browsify runs on your Mac.{/snippet}

  <section class="handler-card glass-panel" aria-labelledby="handler-heading">
    <div>
      <h2 id="handler-heading">Default link handler</h2>
      <p>{!hasConfig ? "Waiting for Browsify to connect." : config.isDefaultBrowser ? "Browsify handles links opened from other apps." : "Make Browsify your Mac’s default browser to route links."}</p>
    </div>
    {#if config.isDefaultBrowser}
      <span class="ready-label"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/></svg>Active</span>
    {:else}
      <button class="app-button" type="button" disabled={!hasConfig} on:click={() => window.finicky.sendMessage({ type: "setDefaultBrowser" })}>Set as default</button>
    {/if}
  </section>

  <section class="browser-card glass-panel" aria-labelledby="browser-heading">
    <div class="card-heading">
      <div><h2 id="browser-heading">Default browser target</h2><p>Used when no handler rule matches a link.</p></div>

    </div>
    <BrowserProfileSelector
      browser={defaultBrowser}
      profile={defaultProfile}
      isCustom={defaultBrowserIsCustom}
      isProfileCustom={defaultProfileIsCustom}
      {installedBrowsers}
      {profilesByBrowser}
      disabled={!$draft || $saving}
      browserPlaceholder="Use built-in default"
      onBrowserChange={(browser, profile, isCustom) => { defaultBrowser = browser; defaultProfile = profile; defaultBrowserIsCustom = isCustom; defaultProfileIsCustom = false; }}
      onProfileChange={(profile, isProfileCustom) => { defaultProfile = profile; defaultProfileIsCustom = isProfileCustom; }}
      onRequestProfiles={(browser) => window.finicky.sendMessage({ type: "getBrowserProfiles", browser })}
      onSave={updateTarget}
      onInput={updateTarget}
    />
  </section>

  <section class="behavior" aria-labelledby="behavior-heading">
    <div class="card-heading"><h2 id="behavior-heading">App behavior</h2></div>
    <div class="options-grid">
      {#if loginItem !== "unavailable"}
        <OptionRow label="Open at Login" hint={loginItem === "needsApproval" ? "Allow Browsify in System Settings → General → Login Items." : "Start Browsify when you sign in to your Mac."} checked={loginItem !== "off"} locked={loginSaving} onchange={() => window.finicky.sendMessage({ type: "setLoginItem", enabled: loginItem === "off" })} />
      {/if}
      <OptionRow label="Keep running" hint="Stay available in the background" bind:checked={keepRunning} locked={!$draft || $saving} {onLockedClick} onchange={() => setOption("keepRunning", keepRunning)} />
      <OptionRow label="Hide menu bar icon" hint="Keep Browsify out of the menu bar" bind:checked={hideIcon} locked={!$draft || $saving} {onLockedClick} onchange={() => setOption("hideIcon", hideIcon)} />
      <OptionRow label="Log URL requests" hint="Write URL handling activity to a log" bind:checked={logRequests} locked={!$draft || $saving} {onLockedClick} onchange={() => setOption("logRequests", logRequests)} />
      <OptionRow label="Automatic updates" hint="Check daily and download signed updates. Ask before restarting." bind:checked={checkForUpdates} locked={!$draft || $saving} {onLockedClick} onchange={() => setOption("checkForUpdates", checkForUpdates)} />
    </div>
    <div class="settings-actions">
      {#if hasOverrides}<button class="app-button" type="button" disabled={!$draft || $saving} on:click={resetSettings}>Reset to defaults</button>{/if}

    </div>
  </section>

  <section class="browser-card glass-panel" aria-labelledby="updates-heading">
    <div class="card-heading">
      <div>
        <h2 id="updates-heading">Browsify updates</h2>
        <p aria-live="polite">{updateInfo?.error ?? (updateInfo?.status === "checking" ? "Checking for updates…" : updateInfo?.status === "downloading" ? "Downloading and verifying update…" : updateInfo?.status === "ready" ? `${updateInfo.version} is ready to install.` : updateInfo?.status === "upToDate" ? "You have the latest stable release." : updateInfo?.hasUpdate ? `${updateInfo.version} is available.` : "Manual checks are always available, even with automatic updates off.")}</p>
      </div>
      <button class="app-button" type="button" disabled={!hasConfig || updateChecking || updateInfo?.status === "checking" || updateInfo?.status === "downloading"} on:click={() => window.finicky.sendMessage({ type: "checkForUpdates" })}>Check for Updates…</button>
    </div>
    {#if updateInfo?.status === "ready"}
      <button class="app-button" type="button" on:click={() => window.finicky.sendMessage({ type: "restartToUpdate" })}>Restart to Update</button>
    {:else if updateInfo?.hasUpdate && (updateInfo.status === "available" || updateInfo.status === "skipped" || updateInfo.status === "error")}
      <button class="app-button" type="button" on:click={() => window.finicky.sendMessage({ type: "downloadUpdate" })}>Download Update</button>
    {/if}
    <a href={updateInfo?.releaseUrl || "https://github.com/dungsnowaxe/browsify/releases"} target="_blank" rel="noopener noreferrer">Release notes and manual download ↗</a>
  </section>
</PageContainer>

<style>
  .handler-card, .browser-card { padding: 20px; }
  .handler-card { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  h2 { margin: 0; font-size: 16px; font-weight: 500; letter-spacing: -0.02em; }
  p { margin: 4px 0 0; color: var(--text-secondary); font-size: 14px; }
  .handler-card button { flex-shrink: 0; }
  .ready-label { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--log-success); white-space: nowrap; }
  .card-heading { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
  .browser-card { display: flex; flex-direction: column; gap: 20px; }
  .settings-actions { display: flex; flex-wrap: wrap; gap: 12px; }
  .behavior { display: flex; flex-direction: column; gap: 16px; }
  .options-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  @media (max-width: 800px) {
    .handler-card, .card-heading { align-items: flex-start; flex-direction: column; gap: 12px; }
    .options-grid { grid-template-columns: 1fr; }
    .handler-card, .browser-card { padding: 20px; }
  }
</style>
