<script lang="ts">
  import { onMount } from "svelte";
  import { Link } from "svelte-routing";
  import UrlTester from "../components/UrlTester.svelte";
  import BrowserIcon from "../components/BrowserIcon.svelte";
  import ImportConfig from "../components/ImportConfig.svelte";
  import type { Choice, UpdateInfo, ConfigInfo } from "../types";

  onMount(() => {
    if (typeof document !== "undefined") {
      document.title = "Home — Browsify";
    }
  });

  export let hasConfig: boolean;
  export let connectionStatus: "connecting" | "connected" | "disconnected" = "connecting";
  export let numErrors: number;
  export let config: ConfigInfo;
  export let updateInfo: UpdateInfo | null;
  export let installedBrowsers: string[] = [];
  export let browserChoices: Choice[] | null = null;
  export let onRetry: () => void = () => {};

  $: defaultBrowser = config.defaultBrowser;
  $: defaultProfile = config.defaultProfile;
  $: handlerCount = config.handlers ?? 0;
  $: rewriteCount = config.rewrites ?? 0;
  $: totalRules = handlerCount + rewriteCount;
  $: handlerShare = totalRules ? (handlerCount / totalRules) * 100 : 0;
  $: setupDone = Number(config.isDefaultBrowser === true) + Number(!!defaultBrowser);
  $: setupPct = hasConfig ? setupDone * 50 : 0;
  $: setupReady = hasConfig && setupDone === 2;
</script>

<div class="dashboard">
  <ImportConfig mode="banner" connected={hasConfig} {browserChoices} {installedBrowsers} />

  <div class="dashboard-top">
    <div class="routing-overview">
      <header class="overview-heading">
        <h1>Browser routing,<br />at a glance.</h1>
      </header>

      {#if setupReady}
        <section class="setup-card ready-compact" aria-labelledby="setup-heading">
          <div class="setup-ready-copy">
            <i class="ready-dot" aria-hidden="true"></i>
            <h2 id="setup-heading">Routing setup is ready</h2>
          </div>
          <Link to="/preferences" class="app-button setup-action">Preferences</Link>
        </section>
      {:else}
        <section class="setup-card" aria-labelledby="setup-heading">
          <div class="setup-head">
            <h2 id="setup-heading">
              {#if !hasConfig}
                {connectionStatus === "disconnected" ? "Browsify is not connected" : "Connecting to Browsify"}
              {:else}
                Finish your routing setup
              {/if}
            </h2>
            {#if !hasConfig}
              <button
                class="app-button primary setup-action"
                type="button"
                disabled={connectionStatus === "connecting"}
                on:click={onRetry}
              >
                {connectionStatus === "connecting" ? "Connecting…" : "Retry connection"}
              </button>
            {:else if config.isDefaultBrowser === false}
              <button
                class="app-button primary setup-action"
                type="button"
                on:click={() => window.finicky.sendMessage({ type: "setDefaultBrowser" })}
              >
                Set as default
              </button>
            {:else}
              <Link to="/preferences" class="app-button setup-action">
                Choose browser
              </Link>
            {/if}
          </div>
          {#if !hasConfig && connectionStatus === "disconnected"}
            <p class="reopen-hint">Open Browsify from Applications, Spotlight, or run <code>browsify ui</code>.</p>
          {/if}
          <div class="setup-progress-row">
            <div
              class="setup-track"
              role="progressbar"
              aria-label="Routing setup"
              aria-valuemin="0"
              aria-valuemax="2"
              aria-valuenow={hasConfig ? setupDone : 0}
              aria-valuetext={hasConfig ? `${setupDone} of 2 checks ready` : connectionStatus === "disconnected" ? "Not connected to Browsify" : "Connecting to Browsify"}
            >
              <span style={`transform:scaleX(${setupPct / 100})`}></span>
            </div>
            <span class="setup-count">{hasConfig ? `${setupDone}/2` : "—"}</span>
          </div>
          <div class="setup-checks">
            <span>
              <i class:ready={config.isDefaultBrowser === true} aria-hidden="true"></i>
              Link handler
              <span class="sr-only">{config.isDefaultBrowser === true ? "(ready)" : "(needs setup)"}</span>
            </span>
            <span>
              <i class:ready={!!defaultBrowser} aria-hidden="true"></i>
              Browser Target
              <span class="sr-only">{defaultBrowser ? "(ready)" : "(needs setup)"}</span>
            </span>
          </div>
        </section>
      {/if}
    </div>

    <UrlTester compact connected={connectionStatus === "connected"} />
  </div>

  <section class="routing-summary" aria-labelledby="routing-heading">
    <div class="section-heading">
      <h2 id="routing-heading">Your routing</h2>
    </div>
    <div class="summary-grid">
      <Link to="/rules" class="summary-card rule-summary rule-card-link">
        <svg class="rule-ring" viewBox="0 0 104 104" role="img" aria-label={`${handlerCount} Handler Rules and ${rewriteCount} Rewrite Rules`}>
          <circle class="ring-track" cx="52" cy="52" r="42" />
          {#if totalRules > 0}
            <circle class="ring-handlers" cx="52" cy="52" r="42" pathLength="100" stroke-dasharray={`${handlerShare} 100`} />
            <circle class="ring-rewrites" cx="52" cy="52" r="42" pathLength="100" stroke-dasharray={`${100 - handlerShare} 100`} stroke-dashoffset={-handlerShare} />
          {/if}
          <text x="52" y="57" text-anchor="middle">{hasConfig ? totalRules : "—"}</text>
        </svg>
        <div class="rule-legend">
          <div><span><i class="handler-dot" aria-hidden="true"></i>Handler Rules</span><strong>{hasConfig ? handlerCount : "—"}</strong></div>
          <div><span><i class="rewrite-dot" aria-hidden="true"></i>Rewrite Rules</span><strong>{hasConfig ? rewriteCount : "—"}</strong></div>
        </div>
        <svg class="card-corner-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M7 7h10v10M7 17 17 7" /></svg>
      </Link>

      <article class="summary-card browser-summary">
        <div class="stat-heading"><h3>Browsers</h3><strong>{hasConfig ? installedBrowsers.length : "—"}</strong></div>
        {#if hasConfig && installedBrowsers.length > 0}
          <div class="browser-icons" role="group" aria-label={installedBrowsers.join(", ")}>
            {#each installedBrowsers.slice(0, 3) as browser}
              <span class="browser-tile" title={browser}><BrowserIcon {browser} size={30} /></span>
            {/each}
            {#if installedBrowsers.length > 3}<span class="browser-more">+{installedBrowsers.length - 3}</span>{/if}
          </div>
        {/if}
        <p>
          {hasConfig
            ? installedBrowsers.length
              ? "Available on this Mac"
              : "No browsers reported yet"
            : connectionStatus === "disconnected"
              ? "Unavailable while disconnected"
              : "Finding browsers…"}
        </p>
      </article>

      <Link to="/preferences" class="summary-card destination-card">
        <div class="destination-head">
          <span>Default Browser Target</span>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M7 7h10v10M7 17 17 7" /></svg>
        </div>
        <div class="destination-value">
          <BrowserIcon browser={hasConfig ? defaultBrowser || "" : ""} size={30} />
          <strong>
            {hasConfig
              ? defaultBrowser || "Not configured"
              : connectionStatus === "disconnected"
                ? "Unavailable"
                : "Connecting…"}
          </strong>
        </div>
        <p>
          {hasConfig
            ? defaultProfile || "Use browser default"
            : connectionStatus === "disconnected"
              ? "Connect Browsify to view"
              : "Loading configuration…"}
        </p>
      </Link>
    </div>
  </section>

  {#if numErrors > 0 || updateInfo?.status === "ready" || updateInfo?.hasUpdate}
    <div class="notices">
      {#if numErrors > 0}
        <Link to="/troubleshoot" class="notice error">
          {numErrors} {numErrors === 1 ? "error needs" : "errors need"} attention
          <span>View activity →</span>
        </Link>
      {/if}
      {#if updateInfo?.status === "ready"}
        <Link to="/preferences" class="notice">
          Browsify {updateInfo.version} is ready to install
          <span>Restart to Update →</span>
        </Link>
      {:else if updateInfo?.hasUpdate && updateInfo.status !== "skipped"}
        <Link to="/preferences" class="notice">
          Browsify {updateInfo.version} is available
          <span>Review update →</span>
        </Link>
      {/if}
    </div>
  {/if}
</div>

<style>
  .dashboard { display: flex; flex-direction: column; gap: 18px; }
  .dashboard-top { display: grid; grid-template-columns: minmax(0, 1.1fr) minmax(0, 1fr); gap: 22px; align-items: stretch; }
  .routing-overview { min-width: 0; display: flex; flex-direction: column; gap: 18px; }
  h1 { font: 400 38px/1.12 var(--font-display); letter-spacing: -0.03em; margin: 0; text-wrap: balance; }
  .setup-card { padding: 16px 18px; border-radius: 24px; background: var(--browsify-surface); }
  .setup-card.ready-compact {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    padding: 14px 18px;
  }
  .setup-ready-copy {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .ready-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--log-success);
    flex-shrink: 0;
  }
  .setup-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .setup-head h2, .setup-ready-copy h2 { font-size: 16px; font-weight: 500; line-height: 1.35; max-width: 23ch; margin: 0; }
  .reopen-hint { margin: 8px 0 0; font-size: 13px; color: var(--text-secondary); line-height: 1.45; }
  .reopen-hint code { font-family: var(--font-mono); font-size: 12px; color: var(--text-primary); background: var(--browsify-inset); padding: 1px 5px; border-radius: 4px; }
  .setup-card :global(.setup-action) { min-height: 44px; padding: 8px 16px; font-size: 13px; flex-shrink: 0; }
  .setup-progress-row { display: flex; align-items: center; gap: 14px; margin-top: 12px; }
  .setup-track { flex: 1; height: 12px; background: color-mix(in srgb, var(--browsify-text) 10%, var(--browsify-surface)); border: 1px solid var(--browsify-control-border); box-shadow: var(--browsify-shadow-inset); padding: 2px; border-radius: 999px; overflow: hidden; }
  .setup-track span { display: block; width: 100%; height: 100%; border-radius: 999px; background: var(--accent-color); transform-origin: left; transition: transform 240ms ease; }
  .setup-count { font-size: 13px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .setup-checks { display: flex; gap: 16px; margin-top: 10px; color: var(--text-secondary); font-size: 13px; }
  .setup-checks span { display: flex; align-items: center; gap: 6px; }
  .setup-checks i { width: 6px; height: 6px; border-radius: 50%; border: 1px solid var(--text-secondary); }
  .setup-checks i.ready { border-color: var(--log-success); background: var(--log-success); }
  .routing-summary { display: flex; flex-direction: column; gap: 12px; }
  .section-heading { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .section-heading h2 { margin: 0; font-size: 19px; font-weight: 500; letter-spacing: -0.025em; }
  .summary-grid { display: grid; grid-template-columns: 1.15fr 1fr 1fr; gap: 14px; }
  .summary-card, :global(.destination-card), :global(.rule-card-link) {
    min-width: 0;
    min-height: 140px;
    padding: 16px;
    background: var(--browsify-panel);
    border: 1px solid var(--browsify-border);
    border-radius: 24px;
    box-shadow: var(--browsify-shadow);
  }
  .summary-grid :global(.rule-card-link) {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    text-decoration: none;
    color: var(--text-primary);
    transition: transform 160ms ease, border-color 160ms ease;
  }
  .card-corner-arrow {
    position: absolute;
    top: 14px;
    right: 14px;
    width: 17px;
    height: 17px;
    color: var(--text-secondary);
  }
  .rule-ring { width: 92px; flex: 0 1 92px; min-width: 72px; }
  .rule-ring circle { fill: none; stroke-width: 7px; transform: rotate(-90deg); transform-origin: center; }
  .ring-track { stroke: var(--browsify-inset); }
  .ring-handlers { stroke: var(--text-primary); }
  .ring-rewrites { stroke: var(--accent-color); }
  .rule-ring text { font: 400 24px var(--font-display); fill: var(--text-primary); letter-spacing: -0.03em; }
  .rule-legend { display: flex; flex-direction: column; gap: 10px; }
  .rule-legend div { display: flex; flex-direction: column; gap: 2px; }
  .rule-legend span { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-secondary); }
  .rule-legend strong { margin-left: 11px; font-size: 20px; font-weight: 500; line-height: 1.1; font-variant-numeric: tabular-nums; }
  .rule-legend i { width: 5px; height: 5px; border-radius: 50%; }
  .handler-dot { background: var(--text-primary); }
  .rewrite-dot { background: var(--accent-color); }
  .stat-heading { display: flex; align-items: baseline; justify-content: space-between; }
  .stat-heading h3 { font-size: 13px; font-weight: 400; color: var(--text-secondary); margin: 0; }
  .stat-heading strong { font: 400 28px/1 var(--font-display); letter-spacing: -0.035em; font-variant-numeric: tabular-nums; }
  .browser-icons { display: flex; align-items: center; min-height: 40px; margin: 10px 0 8px; }
  .browser-tile { width: 40px; height: 40px; display: grid; place-items: center; background: var(--browsify-control-solid); border: 3px solid var(--browsify-panel); border-radius: 50%; }
  .browser-tile + .browser-tile { margin-left: -4px; }
  .browser-more { margin-left: 8px; color: var(--text-secondary); font-size: 13px; }
  .browser-summary { display: flex; flex-direction: column; justify-content: space-between; gap: 10px; }
  .browser-summary p { margin: 0; color: var(--text-secondary); font-size: 13px; }
  .summary-grid :global(.destination-card) {
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--browsify-destination-gradient);
    border: 1px solid var(--browsify-border-strong);
    color: #ffffff;
    text-decoration: none;
    transition: transform 160ms ease, border-color 160ms ease;
  }
  @media (hover: hover) and (pointer: fine) {
    .summary-grid :global(.rule-card-link:hover),
    .summary-grid :global(.destination-card:hover) {
      transform: translateY(-1px);
      border-color: var(--browsify-link);
    }
  }
  .destination-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; color: #ecece6; font-size: 13px; }
  .destination-head svg { width: 18px; height: 18px; flex-shrink: 0; }
  .destination-value { display: flex; align-items: center; gap: 10px; margin-top: 10px; }
  .destination-value strong { font-size: 18px; font-weight: 500; letter-spacing: -0.02em; overflow-wrap: anywhere; line-height: 1.15; }
  :global(.destination-card) p { margin: 8px 0 0; font-size: 13px; color: #ecece6; overflow-wrap: anywhere; }
  .notices { display: flex; flex-direction: column; gap: 8px; }
  .notices :global(.notice) { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 44px; padding: 10px 16px; border-radius: 16px; background: var(--browsify-surface); font-size: 13px; text-decoration: none; color: var(--text-primary); }
  .notices :global(.notice.error) { color: var(--log-error); }
  .notices :global(.notice span) { white-space: nowrap; font-weight: 500; }
  @media (max-width: 1100px) {
    .dashboard-top { gap: 18px; }
    h1 { font-size: 34px; }
    .summary-grid { gap: 12px; }
  }
  @media (max-width: 800px) {
    .dashboard-top { grid-template-columns: 1fr; gap: 18px; }
    .summary-grid { grid-template-columns: 1fr; }
    .summary-card, :global(.destination-card), :global(.rule-card-link) { min-height: 136px; padding: 18px; }
    .rule-legend { flex-direction: row; gap: 24px; }
    .rule-ring { flex-basis: 92px; }
  }
  @media (max-width: 500px) {
    h1 { font-size: 28px; }
    .routing-overview { gap: 14px; }
    .setup-card { padding: 16px; }
    .setup-head { gap: 8px; }
    .setup-checks { flex-wrap: wrap; gap: 6px 12px; }
    .section-heading { align-items: flex-start; flex-direction: column; gap: 4px; }
    .rule-legend { flex-direction: column; gap: 10px; }
    .notices :global(.notice) { align-items: flex-start; flex-direction: column; }
  }
  @media (prefers-reduced-motion: reduce) {
    .setup-card :global(.setup-action),
    .summary-grid :global(.destination-card),
    .summary-grid :global(.rule-card-link) { transform: none !important; }
  }
</style>
