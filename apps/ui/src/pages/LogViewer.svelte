<script lang="ts">
  import { Link } from "svelte-routing";
  import PageContainer from "../components/PageContainer.svelte";
  import type { LogEntry } from "../types";
  import LogContent from "../components/LogContent.svelte";
  import Toggle from "../components/Toggle.svelte";
  import { toast } from "../lib/toast";
  import { attrs, materials } from "../design-system/styles";

  const debugLabelId = $props.id();

  let {
    messageBuffer = [],
    connectionStatus = "connecting",
    onClearLogs,
    onRetry = () => {},
  }: {
    messageBuffer: LogEntry[];
    connectionStatus?: "connecting" | "connected" | "disconnected";
    onClearLogs: () => void;
    onRetry?: () => void;
  } = $props();

  let showDebug = $state(localStorage.getItem("showDebugLogs") === "true");
  let errorsOnly = $state(false);

  const errorCount = $derived(
    messageBuffer.filter((entry) => entry.level.toLowerCase() === "error").length,
  );

  const visibleEntries = $derived(
    messageBuffer.filter((entry) => {
      const level = entry.level.toLowerCase();
      if (errorsOnly) return level === "error";
      return showDebug || level !== "debug";
    }),
  );

  const isDisconnectedEmpty = $derived(
    connectionStatus === "disconnected" && messageBuffer.length === 0,
  );

  async function copyLogs() {
    if (!visibleEntries.length) return;
    const logEntries = visibleEntries
      .map((entry) => {
        const time = new Date(entry.time).toISOString();
        const baseMessage = `[${time}] [${entry.level.padEnd(5)}] ${entry.msg}`;
        const extraFields = Object.entries(entry)
          .filter(
            ([key]: [string, any]) =>
              !["level", "msg", "time", "error"].includes(key),
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
        err instanceof Error ? err.message : (err?.toString() ?? "unknown error"),
      );
    }
  }
</script>

<PageContainer title="Activity">
  {#snippet description()}
    Live routing messages from Browsify. Inspect Handler Rule results and diagnose errors.
  {/snippet}

  {#if isDisconnectedEmpty}
    <section
      class={`disconnected-card ${attrs(materials.glassPanel).class ?? ""}`}
      style={attrs(materials.glassPanel).style}
      aria-labelledby="logs-disconnected-heading"
    >
      <h2 id="logs-disconnected-heading">Browsify is not connected</h2>
      <p>
        Open Browsify from Applications, Spotlight, or run <code>browsify ui</code>, then retry to inspect live routing messages.
      </p>
      <button type="button" class="app-button primary" onclick={onRetry}>
        Retry connection
      </button>
    </section>
  {:else}
    <section
      class={`log-shell ${attrs(materials.glassPanel).class ?? ""}`}
      style={attrs(materials.glassPanel).style}
    >
      <div class="log-controls">
        <div class="log-filters">
          <label class="debug-toggle" class:disabled={errorsOnly}>
            <Toggle
              label="Show debug"
              labelledBy={debugLabelId}
              disabled={errorsOnly}
              bind:checked={showDebug}
              onchange={() => localStorage.setItem("showDebugLogs", String(showDebug))}
            />
            <span id={debugLabelId}>Show debug</span>
          </label>
          <button
            type="button"
            class="app-button filter-chip"
            class:active={errorsOnly}
            aria-pressed={errorsOnly}
            onclick={() => (errorsOnly = !errorsOnly)}
          >
            Errors only{#if errorCount > 0} ({errorCount}){/if}
          </button>
        </div>
        <div class="log-actions">
          <button
            type="button"
            class="app-button"
            disabled={!messageBuffer.length}
            onclick={onClearLogs}
          >
            Clear logs
          </button>
          <button
            type="button"
            class="app-button"
            disabled={!visibleEntries.length}
            onclick={copyLogs}
          >
            Copy logs
          </button>
        </div>
      </div>
      {#if visibleEntries.length}
        <LogContent messageBuffer={visibleEntries} showDebug={true} />
      {:else}
        <div class="empty-logs">
          <h2>{errorsOnly ? "No error entries" : "No activity yet"}</h2>
          <p>
            {#if errorsOnly}
              No errors have been recorded in this session.
            {:else}
              Routing messages will appear here while this window is connected. Enable
              <strong>Log URL requests</strong> in <Link to="/preferences">Preferences</Link> to record every handled link.
            {/if}
          </p>
        </div>
      {/if}
    </section>
  {/if}
</PageContainer>

<style>
  .disconnected-card {
    padding: 20px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .disconnected-card h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .disconnected-card p {
    margin: 0;
    font-size: 14px;
    color: var(--text-secondary);
    max-width: 56ch;
  }

  .disconnected-card code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    background: var(--browsify-inset);
    padding: 1px 5px;
    border-radius: 4px;
  }

  .log-shell {
    padding: 18px;
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 14px;
  }

  .empty-logs {
    margin: auto;
    padding: 40px 24px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .empty-logs h2 {
    font: 400 20px var(--font-display);
    letter-spacing: -0.03em;
    margin: 0;
  }

  .empty-logs p {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0;
    max-width: 48ch;
  }

  .log-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .log-filters {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .debug-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
    min-height: 44px;
  }

  .debug-toggle.disabled {
    cursor: default;
  }

  .filter-chip.active {
    border-color: var(--log-error);
    color: var(--log-error);
    background: color-mix(in srgb, var(--log-error) 8%, var(--browsify-control));
  }

  .log-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
</style>
