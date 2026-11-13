<script lang="ts">
  import BrowserIcon from "./BrowserIcon.svelte";
  import LinkIcon from "./icons/Link.svelte";
  import TestIcon from "./icons/Test.svelte";
  import SpinnerIcon from "./icons/Spinner.svelte";
  import { testUrlInput, testUrlResult, testUrlSubmitted, testUrlLoading, testUrlError, submitUrlTest, clearUrlTest } from "../lib/testUrlStore";
  import { normalizeTestUrl } from "../utils/url";

  export let compact = false;
  $: inputId = compact ? "dashboard-url" : "tester-url";
  $: result = $testUrlResult;

  function submitTest() {
    if ($testUrlLoading) return;
    const url = normalizeTestUrl($testUrlInput);
    if (!url) {
      testUrlError.set("Enter a valid URL, such as example.com.");
      return;
    }
    submitUrlTest(url);
  }

</script>

<section class="url-tester" class:compact aria-label="URL tester">
  <header class="tester-header">
    <span class="tester-tag"><span aria-hidden="true"><TestIcon /></span>URL tester</span>
    <span class="dry-run">Dry run only</span>
  </header>

  <div class="conversation" aria-live="polite" aria-busy={$testUrlLoading}>
    {#if $testUrlSubmitted}
      <div class="url-bubble"><span>Test this link</span><p>{$testUrlSubmitted}</p></div>
      {#if $testUrlLoading}
        <div class="response-bubble loading"><SpinnerIcon /><span>Checking your configuration…</span></div>
      {:else if result && "error" in result}
        <div class="response-bubble failure"><strong>This URL could not be resolved</strong><p>{result.error}</p></div>
      {:else if result}
        <div class="response-bubble">
          <p class="response-label">This link opens in</p>
          <div class="browser-target"><BrowserIcon browser={result.browser} size={32} /><strong>{result.browser || "No browser target"}</strong></div>
          <p class="profile-name">{result.profile || "Browser default profile"}</p>
        </div>
        <div class="result-details">
          <span><i></i>{result.openInBackground ? "In the background" : "In the foreground"}</span>
          <span>No link opened</span>
        </div>
        {#if result.url !== $testUrlSubmitted || !compact}
          <div class="final-url"><span>Final URL</span><p>{result.url}</p></div>
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
        <h2>Check a link.<br />See its destination.</h2>
        <p>Test your rules and browser target.<br />Nothing will be opened.</p>
      </div>
    {/if}
  </div>

  <form class="tester-form" on:submit|preventDefault={submitTest}>
    <div class="input-heading"><label for={inputId}>URL to test</label>{#if $testUrlSubmitted && !$testUrlLoading}<button class="clear-test" type="button" on:click={clearUrlTest}>Clear test</button>{/if}</div>
    <div class="url-field" class:invalid={!!$testUrlError}>
      <input id={inputId} type="text" inputmode="url" placeholder="Paste a link…" autocapitalize="off" autocomplete="off" spellcheck="false" bind:value={$testUrlInput} aria-invalid={!!$testUrlError} aria-describedby={$testUrlError ? `${inputId}-error` : undefined} />
      <button class="submit-test" type="submit" aria-label="Test URL" title="Test URL" disabled={$testUrlLoading}>
        {#if $testUrlLoading}<SpinnerIcon />{:else}<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12h14m-6-6 6 6-6 6" /></svg>{/if}
      </button>
    </div>
    {#if $testUrlError}<p class="input-error" id={`${inputId}-error`} role="alert">{$testUrlError}</p>{/if}
  </form>
</section>

<style>
  .url-tester { min-width: 0; min-height: 440px; padding: 22px; display: flex; flex-direction: column; gap: 18px; border-radius: 30px; background: var(--browsify-surface); background-image: radial-gradient(ellipse at 50% -15%, color-mix(in srgb, var(--accent-color) 13%, transparent), transparent 65%); }
  .url-tester.compact { height: 404px; min-height: 0; padding: 20px; gap: 16px; }
  .tester-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .tester-tag { display: inline-flex; align-items: center; gap: 6px; padding: 7px 12px; background: var(--accent-color); color: var(--browsify-on-accent); font-size: 13px; font-weight: 500; border-radius: 999px; }
  .tester-tag > span { display: flex; }
  .tester-tag :global(svg) { width: 15px; height: 15px; }
  .dry-run { color: var(--text-secondary); font-size: 13px; white-space: nowrap; }
  .conversation { display: flex; flex: 1; flex-direction: column; justify-content: flex-start; gap: 12px; min-height: 180px; max-height: 300px; overflow-y: auto; scrollbar-width: thin; padding: 2px; }
  .compact .conversation { min-height: 0; max-height: none; }
  .tester-empty { text-align: center; padding: 8px 0; margin-block: auto; }
  .test-flow { display: flex; align-items: center; justify-content: center; margin-bottom: 20px; }
  .flow-node { display: grid; place-items: center; width: 48px; height: 48px; border-radius: 16px; color: var(--text-secondary); background: var(--browsify-control); box-shadow: var(--browsify-shadow-control); }
  .flow-node.target { color: var(--accent-color); }
  .flow-node :global(svg) { width: 22px; height: 22px; }
  .flow-line { width: 44px; height: 20px; margin: 0 10px; color: var(--text-secondary); opacity: 0.5; }
  .tester-empty h2 { font: 400 20px/1.3 var(--font-display); letter-spacing: -0.04em; margin: 0; }
  .tester-empty p { margin: 10px 0 0; font-size: 14px; color: var(--text-primary); line-height: 1.5; }
  .url-bubble { align-self: flex-end; max-width: 90%; padding: 10px 14px; border-radius: 18px 18px 4px 18px; background: color-mix(in srgb, var(--browsify-text) 5%, var(--browsify-surface)); }
  .url-bubble span { font-size: 13px; color: var(--text-secondary); }
  .url-bubble p { font-size: 14px; margin: 3px 0 0; overflow-wrap: anywhere; }
  .response-bubble { align-self: flex-start; max-width: 100%; border-radius: 20px 20px 20px 4px; padding: 16px 18px; background: var(--browsify-control); box-shadow: var(--browsify-shadow-control); }
  .response-label { font-size: 13px; margin: 0 0 8px; color: var(--text-secondary); }
  .browser-target { display: flex; align-items: center; gap: 10px; }
  .browser-target strong { font-size: 18px; font-weight: 500; letter-spacing: -0.02em; overflow-wrap: anywhere; }
  .profile-name { margin: 8px 0 0; color: var(--text-secondary); font-size: 13px; overflow-wrap: anywhere; }
  .response-bubble.loading { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .loading :global(svg) { width: 16px; height: 16px; flex-shrink: 0; }
  .response-bubble.failure { color: var(--log-error); }
  .failure strong { font-size: 14px; font-weight: 500; }
  .failure p { font-size: 13px; margin: 8px 0 0; overflow-wrap: anywhere; }
  .result-details { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 6px 12px; font-size: 13px; color: var(--text-secondary); }
  .result-details span { display: inline-flex; align-items: center; gap: 6px; }
  .result-details i { width: 5px; height: 5px; border-radius: 50%; background: var(--log-success); }
  .final-url { padding: 10px 0; }
  .final-url span { font-size: 13px; color: var(--text-secondary); }
  .final-url p { font-family: var(--font-mono); font-size: 13px; margin: 4px 0 0; overflow-wrap: anywhere; }
  .result-warning { color: var(--log-warning); font-size: 13px; margin: 0; }
  .tester-form { margin-top: auto; }
  .input-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 28px; margin-bottom: 6px; color: var(--text-secondary); font-size: 13px; }
  .clear-test { background: none; border: none; color: var(--browsify-link); cursor: pointer; padding: 4px 0; }
  .clear-test:hover { text-decoration: underline; text-underline-offset: 3px; }
  .url-field { display: flex; align-items: center; gap: 8px; padding: 5px 5px 5px 16px; background: var(--browsify-control); border: 1px solid var(--browsify-border); border-radius: 999px; box-shadow: var(--browsify-shadow-control); }
  .url-field:focus-within { outline: 2px solid var(--browsify-link); outline-offset: 2px; }
  .url-field.invalid { border-color: var(--log-error); }
  input { min-width: 0; width: 100%; padding: 0; border: none; outline: none; background: transparent; color: var(--text-primary); font-size: 14px; }
  input::placeholder { color: var(--text-secondary); }
  .submit-test { flex-shrink: 0; width: 38px; height: 38px; display: grid; place-items: center; padding: 10px; border: none; border-radius: 50%; background: var(--accent-color); color: var(--browsify-on-accent); cursor: pointer; transition: transform 150ms ease; }
  .submit-test :global(svg) { width: 18px; height: 18px; }
  .submit-test:disabled { opacity: 0.5; cursor: wait; }
  .input-error { margin: 8px 0 0; color: var(--log-error); font-size: 13px; }
  @media (hover: hover) and (pointer: fine) { .submit-test:not(:disabled):hover { transform: translateY(-1px); } }
  .submit-test:not(:disabled):active { transform: translateY(1px) scale(0.985); }
  @media (min-width: 801px) and (max-width: 1100px) and (max-height: 800px) {
    .url-tester.compact { height: 384px; }
  }
  @media (max-width: 500px) {
    .url-tester, .url-tester.compact { padding: 16px; border-radius: 24px; }
    .tester-header { align-items: flex-start; flex-direction: column; gap: 8px; }
    .conversation { max-height: none; }
    .compact .conversation { max-height: 300px; }
    .tester-empty h2 { font-size: 18px; }
    .flow-node { width: 40px; height: 40px; }
    .flow-line { width: 28px; margin: 0 8px; }
    .url-field { padding-left: 12px; }
    input { font-size: 16px; }
    .submit-test { width: 44px; height: 44px; }
    .clear-test { min-height: 44px; }
  }
  @media (prefers-reduced-motion: reduce) { .submit-test { transform: none !important; } }
</style>
