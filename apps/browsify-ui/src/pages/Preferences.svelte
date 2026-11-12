<script lang="ts">
  import { onDestroy } from "svelte";
  import PageContainer from "../components/PageContainer.svelte";
  import BrowserProfileSelector from "../components/BrowserProfileSelector.svelte";
  import OptionRow from "../components/OptionRow.svelte";
  import LockIcon from "../components/icons/Lock.svelte";
  import type { ConfigInfo, RulesFile } from "../types";
  import { toast } from "../lib/toast";

  export let hasConfig: boolean;
  export let config: ConfigInfo;
  export let rulesFile: RulesFile;
  export let hasPrimaryConfig: boolean;
  export let installedBrowsers: string[] = [];
  export let profilesByBrowser: Record<string, string[]> = {};

  let saveTimer: ReturnType<typeof setTimeout>;
  let pendingSave = false;
  let keepRunning = true;
  let hideIcon = false;
  let logRequests = false;
  let checkForUpdates = false;
  let defaultBrowser = "";
  let defaultProfile = "";
  let defaultBrowserIsCustom = false;
  let defaultProfileIsCustom = false;

  // Sync server updates, not local edits (an empty custom field is a valid edit state).
  function syncPreferences(nextConfig: ConfigInfo, nextRules: RulesFile, primary: boolean) {
    const options = primary ? nextConfig.options : { ...nextConfig.options, ...nextRules.options };
    keepRunning = options?.keepRunning ?? true;
    hideIcon = options?.hideIcon ?? false;
    logRequests = options?.logRequests ?? false;
    checkForUpdates = options?.checkForUpdates ?? false;
    defaultBrowser = primary ? (nextConfig.defaultBrowser ?? "") : nextRules.defaultBrowser || nextConfig.defaultBrowser || "Safari";
    defaultProfile = primary ? "" : nextRules.defaultProfile ?? "";
    defaultBrowserIsCustom = defaultBrowser !== "" && !installedBrowsers.includes(defaultBrowser);
    defaultProfileIsCustom = defaultProfile !== "" && !(profilesByBrowser[defaultBrowser] ?? []).includes(defaultProfile);
  }
  function reconcileChoices(browsers: string[], profiles: Record<string, string[]>) {
    if (defaultBrowser) defaultBrowserIsCustom = !browsers.includes(defaultBrowser);
    if (defaultProfile) defaultProfileIsCustom = !(profiles[defaultBrowser] ?? []).includes(defaultProfile);
  }
  $: syncPreferences(config, rulesFile, hasPrimaryConfig);
  $: reconcileChoices(installedBrowsers, profilesByBrowser);
  $: if (defaultBrowser && !defaultBrowserIsCustom && profilesByBrowser[defaultBrowser] === undefined) {
    window.finicky.sendMessage({ type: "getBrowserProfiles", browser: defaultBrowser });
  }

  function onLockedClick() {
    if (!hasConfig) {
      toast.show("Waiting for Browsify", "info", "Open the Browsify app to connect and edit settings.");
      return;
    }
    toast.show("Primary Config is active", "info", "These settings are managed by your Primary Config (.toml/.js/.ts). Edit that file to change them.");
  }

  function save() {
    if (hasPrimaryConfig || !hasConfig) return;
    clearTimeout(saveTimer);
    pendingSave = false;
    window.finicky.sendMessage({
      type: "saveRules",
      payload: { ...rulesFile, defaultBrowser, defaultProfile, options: { keepRunning, hideIcon, logRequests, checkForUpdates } },
    });
  }

  function scheduleSave() {
    if (hasPrimaryConfig || !hasConfig) return;
    clearTimeout(saveTimer);
    pendingSave = true;
    saveTimer = setTimeout(save, 500);
  }

  onDestroy(() => {
    clearTimeout(saveTimer);
    if (pendingSave) save();
  });
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
      {#if hasPrimaryConfig}<span class="managed"><LockIcon />Primary Config</span>{/if}
    </div>
    <BrowserProfileSelector
      browser={defaultBrowser}
      profile={defaultProfile}
      isCustom={defaultBrowserIsCustom}
      isProfileCustom={defaultProfileIsCustom}
      {installedBrowsers}
      {profilesByBrowser}
      disabled={hasPrimaryConfig || !hasConfig}
      hideProfile={hasPrimaryConfig}
      browserPlaceholder="System default"
      onBrowserChange={(browser, profile, isCustom) => { defaultBrowser = browser; defaultProfile = profile; defaultBrowserIsCustom = isCustom; defaultProfileIsCustom = false; }}
      onProfileChange={(profile, isProfileCustom) => { defaultProfile = profile; defaultProfileIsCustom = isProfileCustom; }}
      onRequestProfiles={(browser) => window.finicky.sendMessage({ type: "getBrowserProfiles", browser })}
      onSave={save}
      onInput={scheduleSave}
    />
  </section>

  <section class="behavior" aria-labelledby="behavior-heading">
    <div class="card-heading"><h2 id="behavior-heading">App behavior</h2><span class="save-note">{hasPrimaryConfig ? "Managed by Primary Config" : "Changes save automatically"}</span></div>
    <div class="options-grid">
      <OptionRow label="Keep running" hint="Stay available in the background" bind:checked={keepRunning} locked={hasPrimaryConfig || !hasConfig} onLockedClick={onLockedClick} onchange={scheduleSave} />
      <OptionRow label="Hide menu bar icon" hint="Keep Browsify out of the menu bar" bind:checked={hideIcon} locked={hasPrimaryConfig || !hasConfig} onLockedClick={onLockedClick} onchange={scheduleSave} />
      <OptionRow label="Log URL requests" hint="Write URL handling activity to a log" bind:checked={logRequests} locked={hasPrimaryConfig || !hasConfig} onLockedClick={onLockedClick} onchange={scheduleSave} />
      <OptionRow label="Check for updates" hint="Check for new Browsify releases" bind:checked={checkForUpdates} locked={hasPrimaryConfig || !hasConfig} onLockedClick={onLockedClick} onchange={scheduleSave} />
    </div>
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
  .managed { display: flex; align-items: center; gap: 6px; color: var(--text-secondary); font-size: 13px; }
  .managed :global(svg) { width: 14px; height: 14px; }
  .behavior { display: flex; flex-direction: column; gap: 16px; }
  .save-note { color: var(--text-secondary); font-size: 13px; }
  .options-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  @media (max-width: 800px) {
    .handler-card, .card-heading { align-items: flex-start; flex-direction: column; gap: 12px; }
    .options-grid { grid-template-columns: 1fr; }
    .handler-card, .browser-card { padding: 20px; }
  }
</style>
