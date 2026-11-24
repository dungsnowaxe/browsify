<script lang="ts">
  import { onMount } from "svelte";
  import { draft, refresh, rpc } from "../lib/configDraft";
  import { equal } from "../lib/configMerge";
  let sources: string[] = [];
  let result: {
    id: string;
    report: string;
    preview: { text: string; needsDefault: boolean };
  } | null = null;
  let browser = "",
    error = "",
    busy = false,
    dismissed = false,
    applied = false;
  async function review() {
    busy = true;
    error = "";
    try {
      result = (await rpc("previewImport")).message;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function apply() {
    if (!result) return;
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
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  onMount(() => {
    rpc("findImport")
      .then((r) => (sources = r.message))
      .catch((e) => (error = String(e)));
  });
</script>

{#if !dismissed && (sources.length || error)}
  <section class="import glass-panel">
    <h2>
      {applied
        ? "Imported configuration is active"
        : "Import existing configuration"}
    </h2>
    <p>{sources.join(" · ")}</p>
    {#if error}<p role="alert">{error}</p>{/if}
    {#if result}
      <pre>{result.report}</pre>
      <button
        class="app-button"
        on:click={async () => {
          try {
            await navigator.clipboard.writeText(result!.report);
          } catch {
            error = "Select the report text and copy it.";
          }
        }}
      >
        Copy report
      </button>
      {#if !applied}<details>
          <summary>Preview config.toml</summary>
          <pre>{result.preview.text}</pre>
        </details>
        {#if result.preview.needsDefault}<label>
            Replacement default browser <input bind:value={browser} />
          </label>{/if}
        {#if $draft && !equal($draft.current, $draft.original)}<p>
            Save or discard your current edits before importing.
          </p>{/if}
        <button
          class="app-button"
          disabled={busy ||
            (result.preview.needsDefault && !browser.trim()) ||
            !!($draft && !equal($draft.current, $draft.original))}
          on:click={apply}
        >
          Use imported config
        </button>{/if}
    {/if}
    {#if !applied}<button class="app-button" disabled={busy} on:click={review}>
        {result ? "Refresh preview" : "Review import"}
      </button>{/if}
    <button
      class="app-button"
      disabled={busy}
      on:click={() => (dismissed = true)}
    >
      {applied ? "Done" : "Cancel"}
    </button>
  </section>
{/if}

<style>
  .import {
    padding: 16px;
    margin-bottom: 16px;
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 18rem;
    overflow: auto;
  }
  button,
  details {
    margin: 0.4rem 0.4rem 0.4rem 0;
  }
</style>
