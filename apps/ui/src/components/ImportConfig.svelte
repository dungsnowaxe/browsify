<script lang="ts">
  import { onMount } from "svelte";
  import BrowserTargetPicker from "./BrowserTargetPicker.svelte";
  import { discard, draft, refresh, rpc } from "../lib/configDraft";
  import { equal } from "../lib/configMerge";
  import { askConfirmImport } from "../lib/draftDialog";
  import { toast } from "../lib/toast";
  import type { Choice } from "../types";

  let {
    mode = "banner",
    connected = true,
    browserChoices = null,
    installedBrowsers = [],
  }: {
    mode?: "banner" | "settings";
    connected?: boolean;
    browserChoices?: Choice[] | null;
    installedBrowsers?: string[];
  } = $props();

  let sources = $state<string[]>([]);
  let result = $state<{
    id: string;
    report: string;
    preview: { text: string; needsDefault: boolean };
  } | null>(null);
  let browser = $state("");
  let error = $state("");
  let busy = $state(false);
  let dismissed = $state(false);
  let applied = $state(false);
  let checked = $state(false);

  const hasDirtyDraft = $derived(!!($draft && !equal($draft.current, $draft.original)));

  function cleanError(err: unknown): string {
    const raw = err instanceof Error ? err.message : String(err);
    return raw.replace(/^Error:\s*/, "").trim() || "Something went wrong. Check that Browsify is running and try again.";
  }

  function friendlySource(raw: string): string {
    const lower = raw.toLowerCase();
    if (lower.includes("finicky")) return "Finicky configuration";
    if (lower.includes("rules.json") || lower.includes("settings.json") || lower.includes("browsify")) {
      return "Previous Browsify configuration";
    }
    if (lower.endsWith(".ts") || lower.endsWith(".js")) return "JavaScript/TypeScript configuration";
    if (lower.endsWith(".json")) return "JSON configuration";
    return "Existing configuration";
  }

  const friendlySources = $derived([...new Set(sources.map(friendlySource))]);

  async function findSources() {
    busy = true;
    error = "";
    try {
      const reply = await rpc("findImport");
      sources = Array.isArray(reply.message) ? reply.message : [];
    } catch (e) {
      error = cleanError(e);
    } finally {
      checked = true;
      busy = false;
    }
  }

  async function review() {
    busy = true;
    error = "";
    try {
      result = (await rpc("previewImport")).message;
    } catch (e) {
      error = cleanError(e);
    } finally {
      busy = false;
    }
  }

  async function apply() {
    if (!result) return;
    const sourceLabel = friendlySources.length
      ? friendlySources.join(" and ")
      : "this imported configuration";
    const confirmed = await askConfirmImport(sourceLabel);
    if (!confirmed) return;
    busy = true;
    error = "";
    try {
      const reply = await rpc("approveImport", {
        id: result.id,
        defaultBrowser: browser,
      });
      window.finicky.receiveMessage({ type: "config", message: reply.message });
      applied = true;
      await refresh();
      toast.success("Imported configuration is now active");
    } catch (e) {
      error = cleanError(e);
    } finally {
      busy = false;
    }
  }

  async function copyReport() {
    if (!result) return;
    try {
      await navigator.clipboard.writeText(result.report);
      toast.success("Import report copied to clipboard");
    } catch {
      error = "Could not access the clipboard. Select the report text below and copy it manually.";
    }
  }

  onMount(() => {
    if (connected) void findSources();
  });

  $effect(() => {
    if (connected && !checked && !busy) {
      void findSources();
    }
  });

  const visible = $derived(
    mode === "settings" || (connected && !dismissed && (sources.length > 0 || !!error)),
  );
</script>

{#if visible}
  <section class="import glass-panel" aria-labelledby={`import-heading-${mode}`}>
    <div class="import-head">
      <div>
        {#if mode === "settings"}
          <h3 class="import-title" id={`import-heading-${mode}`}>
            {applied
              ? "Imported configuration is active"
              : "Import existing configuration"}
          </h3>
        {:else}
          <h2 class="import-title" id={`import-heading-${mode}`}>
            {applied
              ? "Imported configuration is active"
              : "Import existing configuration"}
          </h2>
        {/if}
        {#if friendlySources.length}
          <p class="import-summary">Detected: {friendlySources.join(" · ")}</p>
        {:else if mode === "settings" && checked && !error}
          <p class="import-summary">
            No Finicky or previous Browsify configurations were detected on this Mac.
          </p>
        {:else if mode === "settings"}
          <p class="import-summary">
            Review and migrate an existing Finicky or previous Browsify configuration into your Primary Config.
          </p>
        {/if}
      </div>
      {#if mode === "settings" && !applied && !sources.length}
        <button type="button" class="app-button" disabled={busy} onclick={findSources}>
          {busy ? "Checking…" : "Check again"}
        </button>
      {/if}
    </div>

    {#if error}
      <div class="import-error" role="alert">
        <span>{error}</span>
        <button type="button" class="dismiss-inline" onclick={() => (error = "")}>Dismiss</button>
      </div>
    {/if}

    {#if result}
      <div class="report-block">
        <pre class="import-pre">{result.report}</pre>
        <button type="button" class="app-button" onclick={copyReport}>
          Copy report
        </button>
      </div>

      {#if !applied}
        <details class="preview-details">
          <summary>
            <svg class="disclosure-chevron" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="m6 4 4 4-4 4" />
            </svg>
            Preview Primary Config
          </summary>
          <pre class="import-pre">{result.preview.text}</pre>
        </details>

        {#if result.preview.needsDefault}
          <div class="replacement-picker">
            <p class="import-summary">
              The imported Default Browser Target could not be converted automatically. Choose a replacement Browser Target:
            </p>
            <BrowserTargetPicker
              {browser}
              {browserChoices}
              {installedBrowsers}
              disabled={busy}
              required={true}
              hideProfile={true}
              allowCustom={true}
              browserPlaceholder="Choose replacement Browser Target"
              onBrowserChange={(next) => (browser = next)}
            />
          </div>
        {/if}

        {#if hasDirtyDraft}
          <div class="import-warning-row" role="status">
            <p class="import-warning">
              Save or discard your unsaved changes before applying an import.
            </p>
            <button type="button" class="dismiss-inline" onclick={discard}>
              Discard unsaved changes
            </button>
          </div>
        {:else}
          <p class="import-summary">
            Applying replaces your active Primary Config rules and preferences.
          </p>
        {/if}
      {/if}
    {/if}

    {#if sources.length > 0 || result}
      <div class="import-actions">
        {#if result && !applied}
          <button
            type="button"
            class="app-button primary"
            disabled={busy || (result.preview.needsDefault && !browser.trim()) || hasDirtyDraft}
            onclick={apply}
          >
            {busy ? "Applying…" : "Use imported config"}
          </button>
        {/if}

        {#if !applied && sources.length > 0}
          <button
            type="button"
            class={result ? "app-button" : "app-button primary"}
            disabled={busy}
            onclick={review}
          >
            {busy ? "Reviewing…" : result ? "Refresh preview" : "Review import"}
          </button>
        {/if}

        {#if mode === "banner" || applied || result}
          <button
            type="button"
            class="app-button"
            disabled={busy}
            onclick={() => {
              if (mode === "banner" && !result) dismissed = true;
              else {
                result = null;
                applied = false;
                if (mode === "banner") dismissed = true;
              }
            }}
          >
            {applied ? "Done" : result && mode === "settings" ? "Cancel" : "Dismiss"}
          </button>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .import {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .import-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .import-title {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .import-summary {
    margin: 4px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
  }

  .import-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--browsify-radius-control);
    border: 1px solid var(--log-error);
    background: color-mix(in srgb, var(--log-error) 8%, var(--browsify-panel));
    color: var(--log-error);
    font-size: 13px;
  }

  .dismiss-inline {
    background: none;
    border: none;
    color: inherit;
    font-weight: 500;
    cursor: pointer;
    min-height: 44px;
    padding: 0 10px;
    border-radius: 10px;
    text-decoration: underline;
    flex-shrink: 0;
  }

  .report-block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }

  .import-pre {
    width: 100%;
    margin: 0;
    padding: 12px 14px;
    border-radius: var(--browsify-radius-control);
    border: 1px solid var(--browsify-border-strong);
    background: var(--browsify-inset);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 18rem;
    overflow: auto;
  }

  .preview-details {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .preview-details summary {
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    font-size: 14px;
    font-weight: 500;
    color: var(--browsify-link);
    list-style: none;
  }

  .preview-details summary::-webkit-details-marker {
    display: none;
  }

  .disclosure-chevron {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    transition: transform 150ms ease;
  }

  .preview-details[open] .disclosure-chevron {
    transform: rotate(90deg);
  }

  .replacement-picker {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .import-warning-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px 12px;
    flex-wrap: wrap;
    color: var(--log-warning);
  }

  .import-warning {
    margin: 0;
    font-size: 13px;
    color: var(--log-warning);
  }

  .import-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
</style>
