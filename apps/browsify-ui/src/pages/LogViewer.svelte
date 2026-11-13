<script lang="ts">
  import PageContainer from "../components/PageContainer.svelte";
  import type { LogEntry } from "../types";
  import LogContent from "../components/LogContent.svelte";
  import { toast } from "../lib/toast";
  import { attrs, materials, ui } from "../design-system/styles";

  let {
    messageBuffer = [],
    onClearLogs,
  }: {
    messageBuffer: LogEntry[];
    onClearLogs: () => void;
  } = $props();

  let showDebug = $state(localStorage.getItem("showDebugLogs") === "true");

  async function copyLogs() {
    const logEntries = messageBuffer
      .filter((entry) => showDebug || entry.level.toLowerCase() !== "debug")
      .map((entry) => {
        const time = new Date(entry.time).toISOString();
        const baseMessage = `[${time}] [${entry.level.padEnd(5)}] ${entry.msg}`;
        const extraFields = Object.entries(entry)
          .filter(
            ([key]: [string, any]) =>
              !["level", "msg", "time", "error"].includes(key)
          )
          .map(([key, value]: [string, any]) => `${key}: ${value}`)
          .join(" | ");
        const parts = [baseMessage];
        if (extraFields) parts.push(extraFields);
        if (entry.error) parts.push(`Error: ${entry.error}`);
        return parts.join(" | ");
      })
      .join("\n");

    try {
      await navigator.clipboard.writeText(logEntries);
      toast.success("Logs copied to clipboard");
    } catch (err) {
      console.error("Failed to copy logs:", err);
      toast.error(
        "Failed to copy logs",
        err instanceof Error ? err.message : (err?.toString() ?? "unknown error")
      );
    }
  }
</script>

<PageContainer eyebrow="Diagnostics" title="Activity">
  {#snippet description()}
    Live routing messages from Browsify. Inspect handler results and diagnose errors.
  {/snippet}

  <section class={`log-shell ${attrs(materials.glassPanel).class ?? ""}`} style={attrs(materials.glassPanel).style} data-glass>
    <div class="log-controls">
      <label class="debug-toggle">
        <input
          type="checkbox"
          bind:checked={showDebug}
          onchange={() => localStorage.setItem("showDebugLogs", String(showDebug))}
        />
        Show debug
      </label>
      <button type="button" {...attrs(ui.button)} onclick={onClearLogs}>Clear logs</button>
      <button type="button" {...attrs(ui.button)} onclick={copyLogs}>Copy logs</button>
    </div>
    {#if messageBuffer.some((entry) => showDebug || entry.level.toLowerCase() !== "debug")}
      <LogContent {messageBuffer} {showDebug} />
    {:else}
      <div class="empty-logs"><h2>No activity yet</h2><p>Routing messages will appear here while this window is connected.</p></div>
    {/if}
  </section>
</PageContainer>

<style>
  .log-shell {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .empty-logs { padding: 64px 24px; text-align: center; }
  .empty-logs h2 { font: 400 20px var(--font-display); letter-spacing: -0.04em; margin: 0 0 8px; }
  .empty-logs p { font-size: 14px; color: var(--text-secondary); margin: 0; }

  .log-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .debug-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 0.85em;
    color: var(--text-secondary);
    user-select: none;
    margin-right: auto;
  }

  .debug-toggle input[type="checkbox"] {
    cursor: pointer;
    accent-color: var(--accent-color);
  }
</style>
