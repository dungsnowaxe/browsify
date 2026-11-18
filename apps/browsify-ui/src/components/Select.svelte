<script lang="ts">
  import type { HTMLSelectAttributes } from "svelte/elements";
  import { attrs, ui } from "../design-system/styles";

  // Callers compose options and supply the label, value, and native events.
  let {
    children, value, warning = false, class: className, style, ...rest
  }: Omit<HTMLSelectAttributes, "multiple" | "size"> & { warning?: boolean } = $props();
</script>

<div class="select-wrapper">
  <select {...rest} {value} class={[attrs(ui.select, warning && ui.selectWarning).class, className]} {style}>
    {@render children?.()}
  </select>
</div>

<style>
  .select-wrapper { position: relative; flex: 1; min-width: 0; }
  .select-wrapper::after {
    content: "";
    position: absolute;
    right: 16px;
    top: 50%;
    transform: translateY(-70%) rotate(45deg);
    width: 5px;
    height: 5px;
    border-right: 1.5px solid var(--text-secondary);
    border-bottom: 1.5px solid var(--text-secondary);
    pointer-events: none;
  }
  .select-wrapper:has(select:disabled)::after { opacity: 0.5; }
</style>
