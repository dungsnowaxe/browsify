<script lang="ts">
  import LockIcon from "./icons/Lock.svelte";
  import Tooltip from "./Tooltip.svelte";
  import Toggle from "./Toggle.svelte";
  import { attrs, ui } from "../design-system/styles";

  const LOCKED_TOOLTIP = "Primary config is active — these settings can't be changed here";

  let {
    label,
    hint,
    checked = $bindable(false),
    locked = false,
    onLockedClick,
    onchange,
  }: {
    label: string;
    hint: string;
    checked: boolean;
    locked?: boolean;
    onLockedClick?: () => void;
    onchange?: () => void;
  } = $props();
</script>

{#snippet inner()}
  <span class="option-info">
    <span class="option-text">
      <span class="option-label">
        {label}
        {#if locked}<span class="lock-inline" aria-hidden="true"><LockIcon /></span>{/if}
      </span>
      <span class="option-hint">{hint}</span>
    </span>
    {#if locked}
      <span class="locked-switch" aria-hidden="true">
        <span {...attrs(ui.switch, checked && ui.checked)}><span {...attrs(ui.thumb, checked && ui.thumbChecked)}></span></span>
      </span>
    {:else}
      <Toggle {label} bind:checked {onchange} />
    {/if}
  </span>
{/snippet}

{#if locked}
  <Tooltip text={LOCKED_TOOLTIP} block onclick={onLockedClick}>
    <span class={`option-row locked ${attrs(ui.inset).class ?? ""}`} style={attrs(ui.inset).style}>
      {@render inner()}
    </span>
  </Tooltip>
{:else}
  <div class={`option-row ${attrs(ui.inset).class ?? ""}`} style={attrs(ui.inset).style}>
    {@render inner()}
  </div>
{/if}

<style>
  .option-row {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    width: 100%;
    text-align: left;
    transition: background 0.2s ease, transform 0.2s ease;
  }

  .option-row.locked {
    cursor: help;
    opacity: 0.92;
  }

  .locked-switch { display: flex; align-items: center; width: 44px; height: 44px; flex-shrink: 0; opacity: 0.5; pointer-events: none; }

  .option-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .option-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .option-label {
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .option-hint {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .lock-inline {
    display: inline-flex;
    opacity: 0.55;
  }

  .lock-inline :global(svg) {
    width: 12px;
    height: 12px;
  }
</style>
