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

  const tipId = $props.id();
  let dismissed = $state(false);
  let hovered = $state(false);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      dismissed = true;
    }
  }

  function handleMouseEnter() {
    hovered = true;
  }

  function handleMouseLeave() {
    hovered = false;
    dismissed = false;
  }

  function resetDismissed() {
    dismissed = false;
  }

  $effect(() => {
    if (!hovered || dismissed) return;
    const onWindowKeydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        dismissed = true;
      }
    };
    window.addEventListener("keydown", onWindowKeydown);
    return () => window.removeEventListener("keydown", onWindowKeydown);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  class="tooltip-anchor"
  class:block
  class:dismissed
  aria-describedby={onclick ? undefined : tipId}
  onkeydown={handleKeydown}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  onfocusout={resetDismissed}
>
  {#if onclick}
    <button type="button" class="trigger" aria-describedby={tipId} {onclick}>
      {@render children()}
    </button>
  {:else}
    {@render children()}
  {/if}
  <span
    id={tipId}
    role="tooltip"
    class={`tooltip-text ${attrs(ui.tooltip).class ?? ""}`}
    style={attrs(ui.tooltip).style}
  >
    {text}
  </span>
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
    border-radius: var(--browsify-radius-control);
  }

  .tooltip-text {
    display: none;
    position: absolute;
    bottom: calc(100% + 6px);
    right: 0;
    pointer-events: auto;
    white-space: normal;
    width: max-content;
    max-width: min(220px, calc(100vw - 64px));
  }

  /* Invisible hover bridge so pointer movement into the tooltip keeps it open (WCAG 1.4.13). */
  .tooltip-text::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 100%;
    height: 8px;
  }

  .tooltip-anchor:not(.dismissed):hover .tooltip-text,
  .tooltip-anchor:not(.dismissed):focus-within .tooltip-text {
    display: block;
  }
</style>
