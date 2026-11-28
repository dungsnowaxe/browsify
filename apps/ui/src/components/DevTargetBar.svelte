<script lang="ts">
  import { onMount } from "svelte";

  interface DevProxyStatus {
    configured: boolean;
    targetOrigin: string | null;
    targetHost: string | null;
  }

  let {
    connectionStatus = "connecting",
    configPath = "",
    dirty = false,
  }: {
    connectionStatus?: "connecting" | "connected" | "disconnected";
    configPath?: string;
    dirty?: boolean;
  } = $props();

  let status = $state<DevProxyStatus | null>(null);
  let expanded = $state(false);
  let manuallyToggled = $state(false);
  let urlInput = $state("");
  let submitting = $state(false);
  let errorMessage = $state("");
  let inputEl = $state<HTMLInputElement | null>(null);

  function formatConfigPath(path: string): string {
    if (!path) return "";
    return path.replace(/^\/Users\/[^/]+/, "~");
  }

  async function loadStatus() {
    try {
      const res = await fetch("/__browsify_dev/status", { cache: "no-store" });
      if (!res.ok) return;
      const data = (await res.json()) as DevProxyStatus;
      status = data;
      if (!data.configured) {
        expanded = true;
      }
    } catch {
      // Ignore if dev middleware is unavailable.
    }
  }

  onMount(() => {
    void loadStatus();
  });

  $effect(() => {
    if (manuallyToggled) return;
    if (status && !status.configured) {
      expanded = true;
    } else if (connectionStatus === "disconnected") {
      expanded = true;
    } else if (connectionStatus === "connected" && status?.configured) {
      expanded = false;
    }
  });

  function toggleExpanded(next: boolean) {
    manuallyToggled = true;
    expanded = next;
    errorMessage = "";
    if (next) {
      queueMicrotask(() => inputEl?.focus());
    }
  }

  async function connectTarget(event: SubmitEvent) {
    event.preventDefault();
    if (submitting) return;
    errorMessage = "";
    submitting = true;
    try {
      const res = await fetch("/__browsify_dev/target", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: urlInput }),
      });
      const data = (await res.json().catch(() => ({}))) as Partial<DevProxyStatus> & {
        error?: string;
      };
      if (!res.ok) {
        errorMessage = data.error ?? `Could not set target (HTTP ${res.status}).`;
        return;
      }
      window.location.reload();
    } catch {
      errorMessage = "Could not reach the Vite dev server proxy.";
    } finally {
      submitting = false;
    }
  }

  async function disconnectTarget() {
    if (submitting) return;
    errorMessage = "";
    submitting = true;
    try {
      await fetch("/__browsify_dev/target", { method: "DELETE" });
      window.location.reload();
    } catch {
      errorMessage = "Could not disconnect dev proxy target.";
      submitting = false;
    }
  }

  const displayConfig = $derived(
    configPath
      ? formatConfigPath(configPath)
      : connectionStatus === "connected"
        ? "Built-in defaults"
        : "",
  );
</script>

{#if status}
  <aside class="dev-dock" aria-label="Browsify development proxy target">
    {#if !expanded}
      <div class="dev-pill">
        <span
          class="dot"
          class:connected={connectionStatus === "connected"}
          class:disconnected={connectionStatus === "disconnected"}
          aria-hidden="true"
        ></span>
        <span class="badge">DEV</span>
        <span class="mono">{status.targetHost ?? "No backend"}</span>
        {#if displayConfig}
          <span class="sep" aria-hidden="true">·</span>
          <span class="config-path" title={configPath || "Built-in defaults"}>
            {displayConfig}
          </span>
        {/if}
        <button type="button" class="pill-action" onclick={() => toggleExpanded(true)}>
          Switch
        </button>
      </div>
    {:else}
      <div class="dev-panel">
        <div class="panel-header">
          <div class="panel-meta">
            <span
              class="dot"
              class:connected={connectionStatus === "connected"}
              class:disconnected={connectionStatus === "disconnected"}
              aria-hidden="true"
            ></span>
            <span class="badge">DEV PROXY</span>
            {#if status.targetHost}
              <span class="mono">{status.targetHost}</span>
              {#if displayConfig}
                <span class="sep" aria-hidden="true">·</span>
                <span class="config-path" title={configPath || "Built-in defaults"}>
                  {displayConfig}
                </span>
              {/if}
            {:else}
              <span class="muted">Not connected to a Browsify backend</span>
            {/if}
          </div>
          <div class="panel-actions">
            {#if status.configured}
              <button
                type="button"
                class="text-action"
                disabled={submitting}
                onclick={disconnectTarget}
              >
                Disconnect
              </button>
            {/if}
            {#if status.configured && connectionStatus === "connected"}
              <button type="button" class="text-action" onclick={() => toggleExpanded(false)}>
                Hide
              </button>
            {/if}
          </div>
        </div>

        <form class="target-form" onsubmit={connectTarget}>
          <input
            bind:this={inputEl}
            bind:value={urlInput}
            type="text"
            class="target-input"
            spellcheck="false"
            autocomplete="off"
            aria-label="Browsify UI URL with session token"
            placeholder="Paste Browsify UI URL (http://127.0.0.1:54321/?token=…)"
          />
          <button type="submit" class="connect-btn" disabled={submitting || !urlInput.trim()}>
            {submitting ? "Connecting…" : status.configured ? "Switch target" : "Connect"}
          </button>
        </form>

        {#if dirty}
          <p class="hint warning">
            Unsaved edits stay scoped to the current session — save or discard first, or switching will reload into a clean session.
          </p>
        {:else if !errorMessage}
          <p class="hint">
            Open Preferences from Browsify’s menu bar (or run <code>browsify ui --no-open</code>) and paste the full URL with <code>?token=…</code>.
          </p>
        {/if}

        {#if errorMessage}
          <p class="hint error" role="alert">{errorMessage}</p>
        {/if}
      </div>
    {/if}
  </aside>
{/if}

<style>
  .dev-dock {
    position: fixed;
    top: 8px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 90;
    max-width: calc(100vw - 24px);
  }

  .dev-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 100%;
    padding: 5px 10px 5px 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--browsify-panel) 94%, transparent);
    border: 1px solid var(--browsify-border-strong);
    box-shadow: 0 8px 24px color-mix(in srgb, var(--browsify-rail) 22%, transparent);
    backdrop-filter: blur(10px);
    font-size: 12px;
    line-height: 1.3;
    color: var(--text-primary);
  }

  .dev-panel {
    width: min(680px, calc(100vw - 24px));
    padding: 10px 12px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--browsify-panel) 96%, transparent);
    border: 1px solid var(--browsify-border-strong);
    box-shadow: 0 14px 36px color-mix(in srgb, var(--browsify-rail) 26%, transparent);
    backdrop-filter: blur(12px);
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
    color: var(--text-primary);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .panel-meta {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1 1 auto;
  }

  .panel-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-secondary);
    flex-shrink: 0;
  }

  .dot.connected {
    background: var(--log-success);
  }

  .dot.disconnected {
    background: var(--log-error);
  }

  .badge {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    padding: 2px 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--browsify-accent) 16%, transparent);
    color: var(--browsify-accent);
    flex-shrink: 0;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 11.5px;
    white-space: nowrap;
  }

  .sep {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .muted {
    color: var(--text-secondary);
  }

  .config-path {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 280px;
  }

  .pill-action,
  .text-action {
    border: 1px solid var(--browsify-control-border);
    background: var(--browsify-control);
    color: var(--text-primary);
    border-radius: 999px;
    padding: 2px 9px;
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    flex-shrink: 0;
  }

  .text-action {
    border-color: transparent;
    background: transparent;
    color: var(--text-secondary);
    padding: 2px 6px;
  }

  .text-action:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .target-form {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .target-input {
    flex: 1 1 auto;
    min-width: 0;
    height: 34px;
    padding: 0 10px;
    border-radius: 9px;
    border: 1px solid var(--browsify-control-border);
    background: var(--browsify-control);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .target-input::placeholder {
    color: var(--text-secondary);
    font-family: var(--font-text);
  }

  .connect-btn {
    height: 34px;
    padding: 0 14px;
    border: none;
    border-radius: 999px;
    background: var(--browsify-accent);
    color: var(--browsify-on-accent);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    flex-shrink: 0;
  }

  .connect-btn:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .hint {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .hint code {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .hint.warning {
    color: var(--log-warning);
  }

  .hint.error {
    color: var(--log-error);
  }

  @media (max-width: 700px) {
    .config-path,
    .sep {
      display: none;
    }

    .target-form {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
