<script lang="ts">
  import PageContainer from "../components/PageContainer.svelte";
  import UrlTester from "../components/UrlTester.svelte";

  export let connectionStatus: "connecting" | "connected" | "disconnected" = "connecting";
  export let onRetry: () => void = () => {};
</script>

<PageContainer title="URL tester">
  {#snippet description()}
    Check the Browser Target, profile or workspace, matched rule, and final URL your Primary Config selects. Dry run only — nothing opens.
  {/snippet}
  {#if connectionStatus === "disconnected"}
    <section class="disconnected-card glass-panel" aria-labelledby="tester-disconnected-heading">
      <h2 id="tester-disconnected-heading">Browsify is not connected</h2>
      <p>
        Open Browsify from Applications, Spotlight, or run <code>browsify ui</code>, then retry to test links against your Primary Config.
      </p>
      <button type="button" class="app-button primary" on:click={onRetry}>
        Retry connection
      </button>
    </section>
  {:else}
    <UrlTester
      showHeader={false}
      diagnostic={true}
      connected={connectionStatus === "connected"}
    />
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
</style>
