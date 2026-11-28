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

  $: normalized = browser.trim().toLowerCase();
  $: icon = icons[normalized];
  $: monogram =
    normalized === "dia" || normalized === "company.thebrowser.dia"
      ? { letter: "D", bg: "#1d1e22", fg: "#ffffff" }
      : normalized === "zen" || normalized === "app.zen-browser.zen"
        ? { letter: "Z", bg: "#2b2a33", fg: "#f28a6b" }
        : null;
</script>

{#if icon}
  <img src={`/icons/${icon}.svg`} alt="" width={size} height={size} loading="lazy" decoding="async" />
{:else if monogram}
  <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
    <rect x="1" y="1" width="30" height="30" rx="8" fill={monogram.bg} />
    <text
      x="16"
      y="21"
      text-anchor="middle"
      fill={monogram.fg}
      font-family="Sora, sans-serif"
      font-size="15"
      font-weight="500"
    >{monogram.letter}</text>
  </svg>
{:else}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <circle cx="12" cy="12" r="10" />
    <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20M2 12h20" />
  </svg>
{/if}

<style>
  img, svg { flex-shrink: 0; object-fit: contain; }
</style>
