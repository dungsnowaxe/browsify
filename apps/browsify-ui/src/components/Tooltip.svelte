<script lang="ts">
  import type { Snippet } from "svelte";
  import { attrs, ui } from "../design-system/styles";

  let {
    children,
    text,
    block = false,
    onclick,
  }: {
    children: Snippet;
    text: string;
    block?: boolean;
    onclick?: () => void;
  } = $props();

  const tipId = `tip-${Math.random().toString(36).slice(2, 9)}`;
</script>

<span class="tooltip-anchor" class:block aria-describedby={tipId}>
  {#if onclick}
    <button type="button" class="trigger" aria-describedby={tipId} {onclick}>
      {@render children()}
    </button>
  {:else}
    {@render children()}
  {/if}
  <span id={tipId} role="tooltip" class={`tooltip-text ${attrs(ui.tooltip).class ?? ""}`} style={attrs(ui.tooltip).style}>{text}</span>
</span>

<style>
  .tooltip-anchor {
    position: relative;
    display: inline-flex;
  }

  .tooltip-anchor.block {
    display: flex;
    width: 100%;
  }

  .trigger {
    display: inherit;
    width: inherit;
    border: 0;
    padding: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: inherit;
    cursor: inherit;
  }

  .tooltip-text {
    display: none;
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    pointer-events: none;
    white-space: normal;
  }

  .tooltip-anchor:hover .tooltip-text,
  .tooltip-anchor:focus-within .tooltip-text {
    display: block;
  }
</style>
