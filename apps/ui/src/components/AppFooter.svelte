<script lang="ts">
  import { useLocation } from "svelte-routing";
  import type { UpdateInfo } from "../types";

  let {
    connectionStatus = "connecting",
    updateInfo = null,
    version = "",
    onRetry = () => {},
  }: {
    connectionStatus?: "connecting" | "connected" | "disconnected";
    updateInfo?: UpdateInfo | null;
    version?: string;
    onRetry?: () => void;
  } = $props();

  const location = useLocation();
</script>

<footer class="footer">
  <span class="connection" role="status" aria-live="polite">
    <i
      class:connected={connectionStatus === "connected"}
      class:disconnected={connectionStatus === "disconnected"}
      aria-hidden="true"
    ></i>
    <span>
      {connectionStatus === "connected"
        ? "Connected"
        : connectionStatus === "disconnected"
          ? "Disconnected"
          : "Connecting…"}
    </span>
  </span>
  {#if connectionStatus === "disconnected" && $location.pathname === "/about"}
    <button type="button" class="footer-retry" onclick={onRetry}>
      Retry
    </button>
  {/if}
  <span class="spacer"></span>
  {#if updateInfo?.status === "upToDate"}
    <span class="up-to-date">Up to date</span>
  {/if}
  {#if version}
    <span class="version">{version}</span>
  {/if}
</footer>

<style>
  .footer {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 6px 0 0;
    border-top: 1px solid var(--browsify-border-strong);
    box-shadow: 0 -6px 14px -10px color-mix(in srgb, var(--browsify-rail) 25%, transparent);
    flex-wrap: wrap;
  }

  .connection {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .connection i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-secondary);
    flex-shrink: 0;
  }

  .connection i.connected {
    background: var(--log-success);
  }

  .connection i.disconnected {
    background: var(--log-error);
  }

  .footer-retry {
    background: none;
    border: none;
    color: var(--browsify-link);
    font-size: 13px;
    font-weight: 500;
    text-decoration: underline;
    cursor: pointer;
    min-height: 44px;
    padding: 0 10px;
    border-radius: 10px;
  }

  .spacer {
    flex: 1;
  }

  .up-to-date {
    color: var(--log-success);
    font-size: 13px;
  }

  .version {
    color: var(--text-secondary);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 700px) {
    .footer {
      gap: 6px;
      padding: 6px 0 0;
    }

    .up-to-date {
      display: none;
    }
  }
</style>
