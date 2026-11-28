<script lang="ts">
  import PageContainer from "../components/PageContainer.svelte";
  import ExternalIcon from "../components/icons/External.svelte";
  import { attrs, materials } from "../design-system/styles";

  let {
    version,
  }: {
    version: string;
  } = $props();
</script>

<PageContainer title="About Browsify">
  {#snippet description()}
    URL routing and browser profile launching for macOS.
  {/snippet}

  <section
    class={`hero ${attrs(materials.glassPanel).class ?? ""}`}
    style={attrs(materials.glassPanel).style}
  >
    <img class="brand-mark" src="/favicon.svg" alt="" width="56" height="56" />
    <div class="app-info">
      <span class="app-name">Browsify</span>
      {#if version}
        <span class="version">Version {version}</span>
      {/if}
    </div>
    <p>
      Open links in the right browser and profile, with routing rules you control.
    </p>
    <div class="hero-actions">
      <a
        href="https://github.com/snowaxe/browsify"
        target="_blank"
        rel="noopener noreferrer"
        class="app-button"
      >
        View on GitHub <ExternalIcon />
      </a>
      <a
        href="https://github.com/snowaxe/browsify#readme"
        target="_blank"
        rel="noopener noreferrer"
        class="app-button"
      >
        Documentation &amp; troubleshooting <ExternalIcon />
      </a>
    </div>
  </section>

  <section
    class={`credits ${attrs(materials.glassPanel).class ?? ""}`}
    style={attrs(materials.glassPanel).style}
  >
    <h2>Credits &amp; attribution</h2>
    <p>
      Browsify development by
      <a href="https://github.com/snowaxe" target="_blank" rel="noopener noreferrer">@snowaxe</a>
      (MIT License)
    </p>
    <p>
      Based on
      <a href="https://github.com/johnste/finicky" target="_blank" rel="noopener noreferrer">Finicky</a>,
      originally created by
      <a href="https://github.com/johnste" target="_blank" rel="noopener noreferrer">John Sterling</a>
      (MIT License)
    </p>
    <p>Logo and visual identity redesigned for Browsify.</p>
    <p class="asset-credits">
      Sora and Figtree are licensed under the SIL Open Font License. Browser marks via SVG Logos (CC0). Product names and logos are trademarks of their owners.
    </p>
  </section>
</PageContainer>

<style>
  .hero,
  .credits {
    padding: 24px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  .brand-mark {
    width: 56px;
    height: 56px;
    border-radius: 16px;
    box-shadow: var(--browsify-shadow-icon);
  }

  .app-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .app-name {
    font-size: 1.5em;
    font-weight: 500;
    letter-spacing: -0.03em;
    color: var(--text-primary);
  }

  .version {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .hero p,
  .credits p {
    margin: 0;
    line-height: 1.6;
    color: var(--text-secondary);
    font-size: 0.95em;
  }

  .credits h2 {
    font-size: 16px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    margin: 0 0 4px;
  }

  .credits a {
    color: var(--browsify-link);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
</style>
