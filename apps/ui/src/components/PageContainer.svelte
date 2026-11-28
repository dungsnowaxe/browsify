<script lang="ts">
  import { onMount, type Snippet } from "svelte";

  let page: HTMLDivElement;
  onMount(() => page.closest<HTMLElement>(".container")?.scrollTo(0, 0));

  let {
    children,
    center = false,
    title,
    description,
  }: {
    children: Snippet;
    center?: boolean;
    title?: string;
    description?: Snippet;
  } = $props();

  $effect(() => {
    if (typeof document !== "undefined") {
      document.title = title ? `${title} — Browsify` : "Browsify — URL routing for macOS";
    }
  });
</script>

<div class="page-container" class:center bind:this={page}>
  {#if title}
    <header class="header-section">
      <h1>{title}</h1>
      {#if description}
        <p class="page-description">{@render description()}</p>
      {/if}
    </header>
  {/if}
  <div class="page-body">
    {@render children()}
  </div>
</div>

<style>
  .page-container {
    width: 100%;
    min-height: 100%;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  .page-container.center {
    align-items: center;
  }

  .header-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h1 {
    margin: 0;
    color: var(--text-primary);
    font: 400 36px/1.15 var(--font-display);
    letter-spacing: -0.03em;
    text-wrap: balance;
  }

  .page-description {
    color: var(--text-secondary);
    margin: 0;
    font-size: 14px;
    max-width: 66ch;
  }

  .page-description :global(code) {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--inset-bg);
    padding: 1px 4px;
    border-radius: 4px;
  }

  .page-body {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 16px;
  }

  @media (max-width: 700px) {
    h1 { font-size: 28px; }
    .page-container { gap: 24px; }
  }
</style>
