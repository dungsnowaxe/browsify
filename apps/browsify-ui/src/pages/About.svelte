<script lang="ts">
  import PageContainer from "../components/PageContainer.svelte";
  import ExternalIcon from "../components/icons/External.svelte";
  import { attrs, materials, ui } from "../design-system/styles";

  let {
    version,
  }: {
    version: string;
  } = $props();
</script>

<PageContainer eyebrow="About" title="Browsify">
  {#snippet description()}
    URL routing and browser profile launching for macOS.
  {/snippet}

  <section class={`hero ${attrs(materials.glassPanel).class ?? ""}`} style={attrs(materials.glassPanel).style} data-glass>
    <img class="brand-mark" src="/favicon.svg" alt="" width="56" height="56" />
    <div class="app-info">
      <span class="app-name">Browsify</span>
      <span class="version">Version {version}</span>
    </div>
    <p>
      Drop-in Finicky v3/v4 compatibility with a hybrid native + QuickJS pipeline
      and zero idle UI RAM.
    </p>
    <a
      href="https://github.com/snowaxe/browsify"
      target="_blank"
      rel="noopener noreferrer"
      {...attrs(ui.button, ui.primary)}
    >
      View on GitHub <ExternalIcon />
    </a>
  </section>

  <section class={`credits ${attrs(materials.glassPanel).class ?? ""}`} style={attrs(materials.glassPanel).style} data-glass>
    <h2>Credits &amp; attribution</h2>
    <p>
      Rust rewrite &amp; hybrid architecture by
      <a href="https://github.com/snowaxe" target="_blank" rel="noopener noreferrer"
        >@snowaxe</a
      >
    </p>
    <p>
      Based on
      <a href="https://github.com/johnste/finicky" target="_blank" rel="noopener noreferrer"
        >Finicky</a
      >, originally created by
      <a href="https://github.com/johnste" target="_blank" rel="noopener noreferrer"
        >John Sterling</a
      >
      (MIT License)
    </p>
    <p>
      Icon designed by
      <a href="https://github.com/uetchy" target="_blank" rel="noopener noreferrer"
        >@uetchy</a
      >
    </p>
    <p class="asset-credits">Sora and Figtree are licensed under the SIL Open Font License. Browser marks via SVG Logos (CC0). Product names and logos are trademarks of their owners.</p>
  </section>
</PageContainer>

<style>
  .hero,
  .credits {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .brand-mark {
    width: 56px;
    height: 56px;
    border-radius: 18px;
    display: grid;
    place-items: center;
    background: var(--browsify-accent);
    color: var(--browsify-on-accent);
    font-weight: 500;
    font-size: 1.4rem;
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
    font-size: 0.85em;
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
    color: var(--text-secondary);
    margin: 0 0 4px;
  }

  a {
    color: var(--accent-color);
    text-decoration: none;
  }

  a:hover {
    opacity: 0.85;
  }
</style>
