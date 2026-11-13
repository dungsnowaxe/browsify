<script lang="ts">
  import { onMount, type Snippet } from "svelte";

  let page: HTMLDivElement;
  onMount(() => page.closest<HTMLElement>(".container")?.scrollTo(0, 0));

  let {
    children,
    center = false,
    title,
    description,
    eyebrow,
  }: {
    children: Snippet;
    center?: boolean;
    title?: string;
    description?: Snippet;
    eyebrow?: string;
  } = $props();
</script>

<div class="page-container" class:center bind:this={page}>
  {#if title}
    <header class="header-section">
      {#if eyebrow}
        <p class="eyebrow">{eyebrow}</p>
      {/if}
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
    gap: 10px;
  }

  .eyebrow {
    margin: 0;
    color: var(--browsify-accent);
    font-size: 13px;
    font-weight: 500;
  }

  h1 {
    margin: 0;
    color: var(--text-primary);
    font: 400 36px/1.15 var(--font-display);
    letter-spacing: -0.05em;
    line-height: 1.15;
  }

  .page-description {
    color: var(--text-secondary);
    margin: 0;
    font-size: 14px;
    max-width: 64ch;
  }

  .page-description :global(code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.9em;
    background: var(--inset-bg);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .page-body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  @media (max-width: 700px) {
    h1 { font-size: 28px; }
    .page-container { gap: 24px; }
  }
</style>
