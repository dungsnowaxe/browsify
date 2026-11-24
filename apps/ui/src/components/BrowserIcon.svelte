<script lang="ts">
  export let browser: string;
  export let size = 32;

  const icons: Record<string, string> = {
    safari: "safari",
    "google chrome": "chrome",
    chrome: "chrome",
    firefox: "firefox",
    "firefox developer edition": "firefox",
    "microsoft edge": "microsoft-edge",
    edge: "microsoft-edge",
    "brave browser": "brave",
    brave: "brave",
    opera: "opera",
  };
  $: icon = icons[browser.toLowerCase()];
</script>

{#if icon}
  <img src={`/icons/${icon}.svg`} alt="" width={size} height={size} loading="lazy" decoding="async" />
{:else}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20M2 12h20"/></svg>
{/if}

<style>
  img, svg { flex-shrink: 0; object-fit: contain; }
</style>
