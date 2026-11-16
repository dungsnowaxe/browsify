<script lang="ts">
  import { Link } from "svelte-routing";
  import UrlTester from "../components/UrlTester.svelte";
  import BrowserIcon from "../components/BrowserIcon.svelte";
  import type { UpdateInfo, ConfigInfo, RulesFile } from "../types";

  export let hasConfig: boolean;
  export let numErrors: number;
  export let config: ConfigInfo;
  export let updateInfo: UpdateInfo | null;
  export let rulesFile: RulesFile;
  export let installedBrowsers: string[] = [];

  $: defaultBrowser = config.defaultBrowserIsDynamic ? "Selected by routing rules" : config.defaultBrowser || rulesFile.defaultBrowser;
  $: defaultProfile = config.defaultProfile ?? rulesFile.defaultProfile;
  $: handlerCount = config.handlers ?? rulesFile.rules.length;
  $: rewriteCount = config.rewrites ?? 0;
  $: totalRules = handlerCount + rewriteCount;
  $: handlerShare = totalRules ? (handlerCount / totalRules) * 100 : 0;
  $: setupDone = Number(config.isDefaultBrowser === true) + Number(!!defaultBrowser);
  $: setupPct = hasConfig ? setupDone * 50 : 0;

</script>

<div class="dashboard">
  <div class="dashboard-top">
    <div class="routing-overview">
      <header class="overview-heading">
        <p class="eyebrow">Browsify</p>
        <h1>Browser routing,<br />at a glance.</h1>
      </header>

      <section class="setup-card" aria-labelledby="setup-heading">
        <div class="setup-head">
          <h2 id="setup-heading">{!hasConfig ? "Waiting for Browsify" : setupDone === 2 ? "Your routing setup is ready" : "Finish your routing setup"}</h2>
          {#if hasConfig && config.isDefaultBrowser === false}
            <button class="setup-action" type="button" aria-label="Set Browsify as the default link handler" title="Set Browsify as default" on:click={() => window.finicky.sendMessage({ type: "setDefaultBrowser" })}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 7h10v10M7 17 17 7" /></svg>
            </button>
          {:else}
            <Link to="/preferences" class="setup-action" aria-label="Open routing settings">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={setupDone === 2 ? "m6 12 4 4 8-8" : "M7 7h10v10M7 17 17 7"} /></svg>
            </Link>
          {/if}
        </div>
        <div class="setup-progress-row">
          <div class="setup-track" role="progressbar" aria-label="Routing setup" aria-valuemin="0" aria-valuemax="2" aria-valuenow={hasConfig ? setupDone : 0} aria-valuetext={hasConfig ? `${setupDone} of 2 checks ready` : "Waiting for Browsify"}>
            <span style={`transform:scaleX(${setupPct / 100})`}></span>
          </div>
          <span class="setup-count">{hasConfig ? `${setupDone}/2` : "—"}</span>
        </div>
        <div class="setup-checks">
          <span><i class:ready={config.isDefaultBrowser === true}></i>Link handler</span>
          <span><i class:ready={!!defaultBrowser}></i>Browser target</span>
        </div>
      </section>


    </div>

    <UrlTester compact />
  </div>

  <section class="routing-summary" aria-labelledby="routing-heading">
    <div class="section-heading">
      <h2 id="routing-heading">Your routing</h2>
      <span class="live-label"><i class:connected={hasConfig}></i>{hasConfig ? "Connected" : "Connecting…"}</span>
    </div>
    <div class="summary-grid">
      <article class="summary-card rule-summary">
        <svg class="rule-ring" viewBox="0 0 104 104" role="img" aria-label={`${handlerCount} handler rules and ${rewriteCount} rewrite rules`}>
          <circle class="ring-track" cx="52" cy="52" r="42" />
          {#if totalRules > 0}
            <circle class="ring-handlers" cx="52" cy="52" r="42" pathLength="100" stroke-dasharray={`${handlerShare} 100`} />
            <circle class="ring-rewrites" cx="52" cy="52" r="42" pathLength="100" stroke-dasharray={`${100 - handlerShare} 100`} stroke-dashoffset={-handlerShare} />
          {/if}
          <text x="52" y="57" text-anchor="middle">{hasConfig ? totalRules : "—"}</text>
        </svg>
        <div class="rule-legend">
          <div><span><i class="handler-dot"></i>Handlers</span><strong>{hasConfig ? handlerCount : "—"}</strong></div>
          <div><span><i class="rewrite-dot"></i>Rewrites</span><strong>{hasConfig ? rewriteCount : "—"}</strong></div>
        </div>
      </article>

      <article class="summary-card browser-summary">
        <div class="stat-heading"><h3>Browsers</h3><strong>{hasConfig ? installedBrowsers.length : "—"}</strong></div>
        <div class="browser-icons" aria-label={installedBrowsers.join(", ") || "No browsers reported"}>
          {#each installedBrowsers.slice(0, 3) as browser}
            <span class="browser-tile" title={browser}><BrowserIcon {browser} size={30} /></span>
          {/each}
          {#if installedBrowsers.length > 3}<span class="browser-more">+{installedBrowsers.length - 3}</span>{/if}
        </div>
        <p>{installedBrowsers.length ? "Available on this Mac" : "No browsers reported yet"}</p>
      </article>

      <Link to="/preferences" class="summary-card destination-card">
        <div class="destination-head"><span>Default target</span><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M7 7h10v10M7 17 17 7" /></svg></div>
        <div class="destination-value"><BrowserIcon browser={defaultBrowser || ""} size={30} /><strong>{defaultBrowser || "Not configured"}</strong></div>
        <p>{defaultProfile || "Browser default profile"}</p>
        <svg class="destination-path" viewBox="0 0 160 40" fill="none" aria-hidden="true"><path d="M4 31h40c18 0 18-23 36-23h65" stroke="currentColor" stroke-width="1.5"/><circle cx="4" cy="31" r="3" fill="currentColor"/><path d="m139 2 6 6-6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </Link>
    </div>
  </section>

  {#if numErrors > 0 || updateInfo?.hasUpdate}
    <div class="notices">
      {#if numErrors > 0}<Link to="/troubleshoot" class="notice error">{numErrors} {numErrors === 1 ? "error needs" : "errors need"} attention <span>View activity →</span></Link>{/if}
      {#if updateInfo?.hasUpdate}<a class="notice" href={updateInfo.downloadUrl} target="_blank" rel="noopener noreferrer">Browsify {updateInfo.version} is available <span>Download update ↗</span></a>{/if}
    </div>
  {/if}
</div>

<style>
  .dashboard { display: flex; flex-direction: column; gap: 24px; }
  .dashboard-top { display: grid; grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr); gap: 28px; align-items: stretch; }
  .routing-overview { min-width: 0; display: flex; flex-direction: column; gap: 24px; }
  .eyebrow { margin: 0 0 14px; color: var(--text-secondary); font-size: 13px; }
  h1 { font: 400 40px/1.14 var(--font-display); letter-spacing: -0.055em; margin: 0; }
  .setup-card { padding: 18px 20px; border-radius: 24px; background: var(--browsify-surface); }
  .setup-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .setup-head h2 { font-size: 16px; font-weight: 500; line-height: 1.35; max-width: 23ch; margin: 0; }
  .setup-head :global(.setup-action) { flex-shrink: 0; display: grid; place-items: center; width: 24px; height: 24px; padding: 6px; border-radius: 50%; border: 4px solid color-mix(in srgb, var(--accent-color) 25%, var(--browsify-surface)); background: var(--accent-color); color: var(--browsify-on-accent); box-sizing: content-box; text-decoration: none; cursor: pointer; transition: transform 150ms ease; }
  .setup-head :global(.setup-action svg) { width: 20px; height: 20px; }
  .setup-progress-row { display: flex; align-items: center; gap: 14px; margin-top: 12px; }
  .setup-track { flex: 1; height: 12px; background: color-mix(in srgb, var(--browsify-text) 8%, var(--browsify-surface)); box-shadow: var(--browsify-shadow-inset); padding: 2px; border-radius: 999px; overflow: hidden; }
  .setup-track span { display: block; width: 100%; height: 100%; border-radius: 999px; background: var(--browsify-control-solid); transform-origin: left; transition: transform 240ms ease; }
  .setup-count { font-size: 13px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .setup-checks { display: flex; gap: 16px; margin-top: 10px; color: var(--text-secondary); font-size: 13px; }
  .setup-checks span { display: flex; align-items: center; gap: 6px; }
  .setup-checks i { width: 6px; height: 6px; border-radius: 50%; border: 1px solid var(--text-secondary); }
  .setup-checks i.ready { border-color: var(--log-success); background: var(--log-success); }
  .routing-summary { display: flex; flex-direction: column; gap: 16px; }
  .section-heading { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .section-heading h2 { margin: 0; font-size: 20px; font-weight: 500; letter-spacing: -0.035em; }
  .live-label { display: inline-flex; align-items: center; gap: 7px; font-size: 13px; color: var(--text-secondary); }
  .live-label i { width: 5px; height: 5px; border-radius: 50%; background: var(--text-secondary); }
  .live-label i.connected { background: var(--log-success); }
  .summary-grid { display: grid; grid-template-columns: 1.15fr 1fr 1fr; gap: 16px; }
  .summary-card, :global(.destination-card) { min-width: 0; min-height: 154px; padding: 20px; background: var(--browsify-panel); border-radius: 24px; box-shadow: var(--browsify-shadow); }
  .rule-summary { display: flex; align-items: center; gap: 16px; padding: 16px; }
  .rule-ring { width: 104px; flex: 0 1 104px; min-width: 72px; }
  .rule-ring circle { fill: none; stroke-width: 7px; transform: rotate(-90deg); transform-origin: center; }
  .ring-track { stroke: var(--browsify-inset); }
  .ring-handlers { stroke: var(--text-primary); }
  .ring-rewrites { stroke: var(--accent-color); }
  .rule-ring text { font: 400 24px var(--font-display); fill: var(--text-primary); letter-spacing: -1px; }
  .rule-legend { display: flex; flex-direction: column; gap: 14px; }
  .rule-legend div { display: flex; flex-direction: column; gap: 2px; }
  .rule-legend span { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-secondary); }
  .rule-legend strong { margin-left: 11px; font-size: 20px; font-weight: 500; line-height: 1.1; font-variant-numeric: tabular-nums; }
  .rule-legend i { width: 5px; height: 5px; border-radius: 50%; }
  .handler-dot { background: var(--text-primary); }
  .rewrite-dot { background: var(--accent-color); }
  .stat-heading { display: flex; align-items: baseline; justify-content: space-between; }
  .stat-heading h3 { font-size: 13px; font-weight: 400; color: var(--text-secondary); margin: 0; }
  .stat-heading strong { font: 400 28px/1 var(--font-display); letter-spacing: -0.06em; }
  .browser-icons { display: flex; align-items: center; min-height: 40px; margin: 14px 0 10px; }
  .browser-tile { width: 40px; height: 40px; display: grid; place-items: center; background: #fff; border: 3px solid var(--browsify-panel); border-radius: 50%; }
  .browser-tile + .browser-tile { margin-left: -4px; }
  .browser-more { margin-left: 8px; color: var(--text-secondary); font-size: 13px; }
  .browser-summary p { margin: 0; color: var(--text-secondary); font-size: 13px; }
  .summary-grid :global(.destination-card) { position: relative; overflow: hidden; display: flex; flex-direction: column; padding-bottom: 42px; background: linear-gradient(145deg, #646660, #3a3c36); color: #fff; text-decoration: none; transition: transform 180ms ease; }
  .destination-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; color: #e1e2dc; font-size: 13px; }
  .destination-head svg { width: 18px; height: 18px; }
  .destination-value { display: flex; align-items: center; gap: 10px; margin-top: 12px; }
  .destination-value strong { font-size: 18px; font-weight: 500; letter-spacing: -0.025em; overflow-wrap: anywhere; line-height: 1.15; }
  :global(.destination-card) p { margin: 8px 0 0; font-size: 13px; color: #e1e2dc; overflow-wrap: anywhere; }
  .destination-path { position: absolute; width: 115px; height: 29px; right: 18px; bottom: 9px; color: #f6a08a; opacity: 0.9; }
  .notices { display: flex; flex-direction: column; gap: 8px; }
  .notices :global(.notice) { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 12px 16px; border-radius: 16px; background: var(--browsify-surface); font-size: 13px; text-decoration: none; color: var(--text-primary); }
  .notices :global(.notice.error) { color: var(--log-error); }
  .notices :global(.notice span) { white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) {
    .setup-head :global(.setup-action:hover) { transform: translateY(-1px); }
  }
  .setup-head :global(.setup-action:active) { transform: translateY(1px) scale(0.985); }
  @media (max-width: 1100px) {
    .dashboard-top { gap: 20px; }
    h1 { font-size: 34px; }
    .summary-grid { gap: 12px; }
    .summary-card, :global(.destination-card) { padding: 16px; }
    .rule-summary { gap: 8px; }
  }
  @media (max-width: 800px) {
    .dashboard-top { grid-template-columns: 1fr; gap: 24px; }
    .summary-grid { grid-template-columns: 1fr; }
    .summary-card, :global(.destination-card) { min-height: 160px; padding: 20px; }
    .rule-summary { justify-content: space-between; }
    .rule-legend { flex-direction: row; gap: 24px; }
    .rule-ring { flex-basis: 104px; }
  }
  @media (min-width: 801px) and (max-width: 1100px) and (max-height: 800px) {
    .routing-overview { gap: 20px; }
    .dashboard { gap: 20px; }
  }
  @media (max-width: 500px) {
    h1 { font-size: 28px; }
    .routing-overview { gap: 20px; }
    .setup-card { padding: 16px; }
    .setup-head { gap: 8px; }
    .setup-head :global(.setup-action) { width: 24px; height: 24px; padding: 6px; }
    .setup-checks { flex-wrap: wrap; gap: 6px 12px; }
    .section-heading { align-items: flex-start; flex-direction: column; gap: 4px; }
    .rule-legend { flex-direction: column; gap: 12px; }
    .notices :global(.notice) { align-items: flex-start; flex-direction: column; }
  }
  @media (prefers-reduced-motion: reduce) {
    .setup-head :global(.setup-action), .summary-grid :global(.destination-card) { transform: none !important; }
  }
</style>
