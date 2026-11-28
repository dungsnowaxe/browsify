<script lang="ts">
  import { Link } from "svelte-routing";
  import BrowserIcon from "./BrowserIcon.svelte";
  import Select from "./Select.svelte";
  import LinkIcon from "./icons/Link.svelte";
  import TestIcon from "./icons/Test.svelte";
  import SpinnerIcon from "./icons/Spinner.svelte";
  import XIcon from "./icons/X.svelte";
  import {
    testUrlInput,
    testUrlResult,
    testUrlSubmitted,
    testOpenerInput,
    testOpenerSubmitted,
    testHistory,
    testUrlLoading,
    testUrlError,
    submitUrlTest,
    clearUrlTest,
    clearTestHistory,
    type TestHistoryEntry,
  } from "../lib/testUrlStore";
  import {
    countInvalidRules,
    draft,
    requestFocusRuleIndex,
    save,
    saving,
  } from "../lib/configDraft";
  import { equal } from "../lib/configMerge";
  import { normalizeTestUrl } from "../utils/url";

  export let compact = false;
  export let showHeader = true;
  export let diagnostic = false;
  export let connected = true;

  const CUSTOM_OPENER = "__custom__";
  const OPENER_PRESETS = [
    { id: "", label: "None (direct link)" },
    { id: "com.tinyspeck.slackmacgap", label: "Slack" },
    { id: "com.apple.mail", label: "Apple Mail" },
    { id: "com.linear", label: "Linear" },
    { id: "notion.id", label: "Notion" },
    { id: "com.hnc.Discord", label: "Discord" },
    { id: "com.microsoft.teams2", label: "Microsoft Teams" },
  ];

  let customOpenerMode = false;

  $: inputId = compact ? "dashboard-url" : "tester-url";
  $: openerId = compact ? "dashboard-opener" : "tester-opener";
  $: result = $testUrlResult;
  $: hasUnsavedDraft = !!($draft && !equal($draft.current, $draft.original));
  $: invalidRuleCount = $draft ? countInvalidRules($draft.current.rows) : 0;
  $: presetOpenerIds = new Set(OPENER_PRESETS.map((p) => p.id));
  $: if ($testOpenerInput && !presetOpenerIds.has($testOpenerInput)) {
    customOpenerMode = true;
  }

  function openerLabel(id: string): string {
    const preset = OPENER_PRESETS.find((p) => p.id === id);
    return preset ? `${preset.label} (${preset.id})` : id;
  }

  function handleOpenerSelect(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    if (value === CUSTOM_OPENER) {
      customOpenerMode = true;
      testOpenerInput.set("");
      return;
    }
    customOpenerMode = false;
    testOpenerInput.set(value);
  }

  function submitTest() {
    if ($testUrlLoading || !connected) return;
    const url = normalizeTestUrl($testUrlInput);
    if (!url) {
      testUrlError.set("Enter a valid URL, such as example.com.");
      return;
    }
    submitUrlTest(url, diagnostic ? $testOpenerInput : "");
  }

  async function saveAndTest() {
    if ($saving || !connected || invalidRuleCount > 0) return;
    const saved = await save();
    if (!saved) return;
    const candidate = normalizeTestUrl($testUrlInput || $testUrlSubmitted);
    if (candidate) {
      submitUrlTest(candidate, diagnostic ? $testOpenerInput : "");
    }
  }

  function rerunHistory(entry: TestHistoryEntry) {
    if ($testUrlLoading || !connected) return;
    testUrlInput.set(entry.url);
    testOpenerInput.set(entry.opener ?? "");
    customOpenerMode = !!(entry.opener && !presetOpenerIds.has(entry.opener));
    submitUrlTest(entry.url, entry.opener ?? "");
  }
</script>

<section class="url-tester" class:compact class:diagnostic aria-label="URL tester">
  {#if showHeader}
    <header class="tester-header">
      <span class="tester-tag"><span aria-hidden="true"><TestIcon /></span>URL tester</span>
      <span class="dry-run">Dry run only</span>
    </header>
  {/if}

  {#if hasUnsavedDraft}
    <div class="draft-notice" role="status">
      <span>Testing your active saved configuration — save changes to test your edits.</span>
      <button
        type="button"
        class="draft-save-btn"
        disabled={$saving || invalidRuleCount > 0 || !connected}
        on:click={saveAndTest}
      >
        {$saving ? "Saving…" : "Save & test"}
      </button>
    </div>
  {/if}

  <div class="conversation" aria-live="polite" aria-busy={$testUrlLoading}>
    {#if $testUrlSubmitted}
      <div class="url-bubble">
        <span>Tested URL</span>
        <p>{$testUrlSubmitted}</p>
        {#if $testOpenerSubmitted}
          <span class="opener-meta">Opened from {openerLabel($testOpenerSubmitted)}</span>
        {/if}
      </div>
      {#if $testUrlLoading}
        <div class="response-bubble loading"><SpinnerIcon /><span>Checking your Primary Config…</span></div>
      {:else if result && "error" in result}
        <div class="response-bubble failure"><strong>This URL could not be resolved</strong><p>{result.error}</p></div>
      {:else if result}
        <div class="response-bubble">
          {#if result.browser === "none"}
            <p class="response-label">Launch suppressed</p>
            <div class="browser-target"><strong>This link won’t open</strong></div>
            <p class="profile-name">Matched a rule set to Don’t open.</p>
          {:else}
            <p class="response-label">Browser Target</p>
            <div class="browser-target">
              <BrowserIcon browser={result.browser} size={32} />
              <strong>{result.browser || "No Browser Target"}</strong>
            </div>
            <p class="profile-name">{result.profile || "Use browser default"}</p>
          {/if}
        </div>
        <div class="result-details">
          {#if result.matchedBy === "handler"}
            {#if result.matchedRuleIndex && !hasUnsavedDraft}
              <Link
                to="/rules"
                class="match-link"
                on:click={() => requestFocusRuleIndex(result.matchedRuleIndex!)}
              >
                <i aria-hidden="true"></i>
                Matched Handler Rule #{result.matchedRuleIndex} →
              </Link>
            {:else if result.matchedRuleIndex}
              <span class="match-attribution">
                <i aria-hidden="true"></i>
                Matched saved Handler Rule #{result.matchedRuleIndex}
              </span>
              <Link to="/rules" class="match-link">View Rules →</Link>
            {:else}
              <span class="match-attribution">
                <i aria-hidden="true"></i>
                Matched Handler Rule
              </span>
            {/if}
          {:else if result.matchedBy === "default"}
            <span class="match-attribution">
              <i class="fallback-dot" aria-hidden="true"></i>
              Fallback · Default Browser Target
            </span>
          {/if}
          {#if result.rewritten}
            <span>URL rewritten</span>
          {/if}
          {#if result.browser !== "none"}
            <span>{result.openInBackground ? "In the background" : "In the foreground"}</span>
          {/if}
          <span>Dry run — no link opened</span>
        </div>
        {#if result.url !== $testUrlSubmitted || !compact}
          <div class="final-url"><span>Final URL</span><p>{result.url}</p></div>
        {/if}
        {#if result.action?.kind === "dia_profile"}
          <p class="profile-name">Dia must have an open window. The exact profile name is checked when opening the link.</p>
        {/if}
        {#if result.profile?.startsWith("workspace:")}
          <p class="profile-name">Zen targets the selected workspace directly. A missing workspace suppresses the open instead of falling back.</p>
        {/if}
        {#if result.warning}<p class="result-warning">{result.warning}</p>{/if}
      {/if}
    {:else}
      <div class="tester-empty">
        <div class="test-flow" aria-hidden="true">
          <span class="flow-node"><LinkIcon /></span>
          <svg class="flow-line" viewBox="0 0 64 20" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M0 10h62m-6-6 6 6-6 6" /></svg>
          <span class="flow-node target"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="4"/><path d="M3 9h18M7 6.5h.01m3 0h.01"/></svg></span>
        </div>
        {#if diagnostic}
          <h2>Check a link and Opener.</h2>
          <p>See which Handler Rule matches, whether a Rewrite Rule runs, and which Browser Target opens.</p>
        {:else}
          <h2>Check a link.<br />See its destination.</h2>
          <p>Test your rules and Browser Target.<br />Nothing will be opened.</p>
        {/if}
      </div>
    {/if}
  </div>

  <form class="tester-form" on:submit|preventDefault={submitTest}>
    {#if diagnostic}
      <div class="opener-field">
        <label for={openerId}>Opener (optional)</label>
        <p class="opener-hint" id={`${openerId}-hint`}>
          App where you clicked the link (used by Opener-scoped Handler Rules).
        </p>
        <div class="opener-control">
          {#if customOpenerMode}
            <input
              id={openerId}
              class="app-input"
              type="text"
              placeholder="Bundle ID, e.g. com.tinyspeck.slackmacgap"
              aria-describedby={`${openerId}-hint`}
              disabled={!connected}
              autocapitalize="off"
              autocomplete="off"
              spellcheck="false"
              bind:value={$testOpenerInput}
            />
            <button
              type="button"
              class="opener-clear"
              disabled={!connected}
              aria-label="Choose a preset Opener"
              on:click={() => {
                customOpenerMode = false;
                testOpenerInput.set("");
              }}
            >
              <XIcon />
            </button>
          {:else}
            <Select
              id={openerId}
              value={$testOpenerInput}
              aria-describedby={`${openerId}-hint`}
              disabled={!connected}
              onchange={handleOpenerSelect}
            >
              {#each OPENER_PRESETS as preset (preset.id)}
                <option value={preset.id}>{preset.label}</option>
              {/each}
              <option value={CUSTOM_OPENER}>Custom bundle ID…</option>
            </Select>
          {/if}
        </div>
      </div>
    {/if}

    <div class="input-heading">
      <label for={inputId}>URL to test</label>
      {#if $testUrlSubmitted && !$testUrlLoading}
        <button class="clear-test" type="button" on:click={clearUrlTest}>Clear test</button>
      {/if}
    </div>
    <div class="url-field" class:invalid={!!$testUrlError} class:disabled={!connected}>
      <input
        id={inputId}
        class="url-input"
        type="text"
        inputmode="url"
        placeholder={connected ? "Paste a link…" : "Connect Browsify to test URLs"}
        disabled={!connected}
        autocapitalize="off"
        autocomplete="off"
        spellcheck="false"
        bind:value={$testUrlInput}
        aria-invalid={!!$testUrlError}
        aria-describedby={$testUrlError ? `${inputId}-error` : undefined}
      />
      <button
        class="submit-test"
        type="submit"
        aria-label="Test URL"
        title={connected ? "Test URL" : "Connect Browsify to test URLs"}
        disabled={$testUrlLoading || !connected}
      >
        {#if $testUrlLoading}
          <SpinnerIcon />
        {:else}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12h14m-6-6 6 6-6 6" /></svg>
        {/if}
      </button>
    </div>
    {#if $testUrlError}<p class="input-error" id={`${inputId}-error`} role="alert">{$testUrlError}</p>{/if}
  </form>

  {#if diagnostic && $testHistory.length > 0}
    <section class="history-section" aria-labelledby="recent-tests-heading">
      <div class="history-header">
        <h2 id="recent-tests-heading">Recent tests</h2>
        <button type="button" class="clear-test" on:click={clearTestHistory}>Clear history</button>
      </div>
      <div class="history-list">
        {#each $testHistory as entry (entry.url + (entry.opener ?? ""))}
          <button
            type="button"
            class="history-item"
            disabled={!connected || $testUrlLoading}
            on:click={() => rerunHistory(entry)}
          >
            <span class="history-url">{entry.url}</span>
            <span class="history-meta">
              {#if entry.opener}
                <span class="history-opener">{openerLabel(entry.opener)}</span>
              {/if}
              <span>
                {entry.browser === "none"
                  ? "Don’t open"
                  : `${entry.browser}${entry.profile ? ` (${entry.profile})` : ""}`}
              </span>
            </span>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</section>

<style>
  .url-tester {
    min-width: 0;
    min-height: 360px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    border-radius: 28px;
    background: var(--browsify-surface);
  }
  .url-tester.compact { height: 356px; min-height: 0; padding: 18px; gap: 12px; }
  .tester-header { display: flex; align-items: center; justify-content: space-between; gap: 8px 12px; flex-wrap: wrap; }
  .tester-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    background: var(--browsify-inset);
    border: 1px solid var(--browsify-border-strong);
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 500;
    border-radius: 999px;
  }
  .tester-tag > span { display: flex; color: var(--browsify-link); }
  .tester-tag :global(svg) { width: 15px; height: 15px; }
  .dry-run { color: var(--text-secondary); font-size: 13px; white-space: nowrap; }
  .draft-notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px 12px;
    flex-wrap: wrap;
    margin: 0;
    padding: 8px 12px;
    border-radius: 12px;
    background: var(--browsify-panel);
    border: 1px solid var(--browsify-control-border);
    color: var(--log-warning);
    font-size: 12px;
    line-height: 1.4;
  }
  .draft-save-btn {
    background: none;
    border: none;
    color: var(--browsify-link);
    font-size: 13px;
    font-weight: 500;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
    min-height: 44px;
    padding: 0 4px;
    flex-shrink: 0;
  }
  .draft-save-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .conversation { display: flex; flex: 1; flex-direction: column; justify-content: flex-start; gap: 12px; min-height: 140px; padding: 2px; }
  .compact .conversation { min-height: 0; overflow-y: auto; scrollbar-width: thin; }
  .tester-empty { text-align: center; padding: 6px 0; margin-block: auto; }
  .test-flow { display: flex; align-items: center; justify-content: center; margin-bottom: 14px; }
  .flow-node { display: grid; place-items: center; width: 44px; height: 44px; border-radius: 14px; color: var(--text-secondary); background: var(--browsify-control); box-shadow: var(--browsify-shadow-control); }
  .flow-node.target { color: var(--accent-color); }
  .flow-node :global(svg) { width: 20px; height: 20px; }
  .flow-line { width: 38px; height: 20px; margin: 0 10px; color: var(--text-secondary); opacity: 0.5; }
  .tester-empty h2 { font: 400 19px/1.3 var(--font-display); letter-spacing: -0.03em; margin: 0; }
  .tester-empty p { margin: 8px 0 0; font-size: 14px; color: var(--text-secondary); line-height: 1.5; }
  .url-bubble { width: 100%; padding: 10px 14px; border-radius: 14px; background: color-mix(in srgb, var(--browsify-text) 5%, var(--browsify-surface)); }
  .url-bubble span { font-size: 12px; color: var(--text-secondary); }
  .url-bubble p { font-size: 14px; margin: 3px 0 0; overflow-wrap: anywhere; }
  .opener-meta { display: block; margin-top: 6px; font-size: 12px; color: var(--text-secondary); }
  .response-bubble { width: 100%; border-radius: 16px; padding: 14px 16px; background: var(--browsify-control); border: 1px solid var(--browsify-border-strong); }
  .response-label { font-size: 12px; margin: 0 0 8px; color: var(--text-secondary); }
  .browser-target { display: flex; align-items: center; gap: 10px; }
  .browser-target strong { font-size: 18px; font-weight: 500; letter-spacing: -0.02em; overflow-wrap: anywhere; }
  .profile-name { margin: 8px 0 0; color: var(--text-secondary); font-size: 13px; overflow-wrap: anywhere; }
  .response-bubble.loading { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .loading :global(svg) { width: 16px; height: 16px; flex-shrink: 0; }
  .response-bubble.failure { color: var(--log-error); border-color: var(--log-error); }
  .failure strong { font-size: 14px; font-weight: 500; }
  .failure p { font-size: 13px; margin: 8px 0 0; overflow-wrap: anywhere; }
  .result-details { display: flex; align-items: center; flex-wrap: wrap; gap: 6px 14px; font-size: 13px; color: var(--text-secondary); }
  .result-details span,
  .result-details :global(.match-link) { display: inline-flex; align-items: center; gap: 6px; }
  .result-details :global(.match-link) {
    color: var(--browsify-link);
    font-weight: 500;
    text-decoration: underline;
    text-underline-offset: 3px;
    min-height: 44px;
  }
  .result-details i { width: 6px; height: 6px; border-radius: 50%; background: var(--log-success); }
  .result-details i.fallback-dot { background: var(--text-secondary); }
  .match-attribution { color: var(--text-primary); font-weight: 500; }
  .final-url { padding: 8px 0; }
  .final-url span { font-size: 12px; color: var(--text-secondary); }
  .final-url p { font-family: var(--font-mono); font-size: 13px; margin: 4px 0 0; overflow-wrap: anywhere; }
  .result-warning { color: var(--log-warning); font-size: 13px; margin: 0; }
  .tester-form { margin-top: auto; display: flex; flex-direction: column; gap: 10px; }
  .opener-field { display: flex; flex-direction: column; gap: 4px; }
  .opener-field label { font-size: 13px; color: var(--text-secondary); }
  .opener-hint { margin: 0 0 2px; font-size: 12px; color: var(--text-secondary); }
  .opener-control { display: flex; align-items: center; gap: 8px; }
  .opener-clear {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: var(--browsify-radius-control);
    border: 1px solid var(--browsify-control-border);
    background: var(--browsify-inset);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .opener-clear:not(:disabled):hover { color: var(--text-primary); }
  .input-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 28px; color: var(--text-secondary); font-size: 13px; }
  .clear-test { background: none; border: none; color: var(--browsify-link); cursor: pointer; padding: 0 4px; min-height: 44px; display: inline-flex; align-items: center; font-size: 13px; font-weight: 500; }
  .clear-test:hover { text-decoration: underline; text-underline-offset: 3px; }
  .url-field { display: flex; align-items: center; gap: 8px; padding: 4px 4px 4px 16px; background: var(--browsify-control); border: 1px solid var(--browsify-control-border); border-radius: 999px; box-shadow: var(--browsify-shadow-control); }
  .diagnostic .url-field { border-radius: var(--browsify-radius-control); padding-left: 14px; }
  .url-field.disabled { background: var(--browsify-inset); border-color: var(--browsify-border-strong); }
  .url-field:has(input:focus-visible) { outline: 2px solid var(--browsify-link); outline-offset: 2px; }
  .url-field.invalid { border-color: var(--log-error); }
  .url-input { min-width: 0; width: 100%; min-height: 44px; align-self: stretch; padding: 0; border: none; outline: none; background: transparent; color: var(--text-primary); font-size: 14px; }
  .url-input::placeholder { color: var(--text-secondary); }
  .url-input:disabled { cursor: default; }
  .submit-test { flex-shrink: 0; width: 44px; height: 44px; display: grid; place-items: center; padding: 10px; border: none; border-radius: 50%; background: var(--accent-color); color: var(--browsify-on-accent); cursor: pointer; transition: transform 150ms ease; }
  .diagnostic .submit-test { border-radius: 10px; }
  .submit-test :global(svg) { width: 18px; height: 18px; }
  .submit-test:disabled {
    background: var(--browsify-inset);
    color: var(--text-secondary);
    border: 1px solid var(--browsify-border-strong);
    cursor: default;
  }
  .input-error { margin: 0; color: var(--log-error); font-size: 13px; }
  .history-section {
    padding-top: 12px;
    border-top: 1px solid var(--browsify-border-strong);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .history-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .history-header h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .history-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .history-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    width: 100%;
    min-height: 44px;
    padding: 8px 12px;
    border-radius: var(--browsify-radius-control);
    border: 1px solid var(--browsify-control-border);
    background: var(--browsify-panel);
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    font-size: 13px;
    transition: background-color 140ms ease, border-color 140ms ease;
  }
  .history-item:not(:disabled):hover {
    background: var(--browsify-inset);
    border-color: var(--browsify-link);
  }
  .history-item:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .history-url {
    font-family: var(--font-mono);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .history-meta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .history-opener {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--browsify-inset);
  }
  @media (hover: hover) and (pointer: fine) { .submit-test:not(:disabled):hover { transform: translateY(-1px); } }
  .submit-test:not(:disabled):active { transform: translateY(1px) scale(0.985); }
  @media (max-width: 800px) {
    .url-tester, .url-tester.compact {
      height: auto;
      min-height: 0;
    }
    .compact .conversation { overflow-y: visible; }
  }
  @media (max-width: 500px) {
    .url-tester, .url-tester.compact { padding: 16px; border-radius: 24px; }
    .tester-empty h2 { font-size: 18px; }
    .flow-node { width: 40px; height: 40px; }
    .flow-line { width: 28px; margin: 0 8px; }
    .url-field { padding-left: 12px; }
    .url-input { font-size: 16px; }
  }
  @media (prefers-reduced-motion: reduce) { .submit-test { transform: none !important; } }
</style>
