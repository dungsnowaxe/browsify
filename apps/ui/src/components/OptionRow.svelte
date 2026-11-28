<script lang="ts">
  import LockIcon from "./icons/Lock.svelte";
  import Tooltip from "./Tooltip.svelte";
  import Toggle from "./Toggle.svelte";
  import { attrs, ui } from "../design-system/styles";

  const uid = $props.id();
  const labelId = `${uid}-label`;
  const hintId = `${uid}-hint`;

  let {
    label,
    hint,
    checked = $bindable(false),
    locked = false,
    flat = false,
    modified = false,
    hintTone = "default",
    lockedText = "Open Browsify to connect and change this setting.",
    onLockedClick,
    onchange,
  }: {
    label: string;
    hint: string;
    checked: boolean;
    locked?: boolean;
    flat?: boolean;
    modified?: boolean;
    hintTone?: "default" | "warning";
    lockedText?: string;
    onLockedClick?: () => void;
    onchange?: () => void;
  } = $props();
</script>

{#snippet inner()}
  <span class="option-info">
    <span class="option-text">
      <span id={labelId} class="option-label">
        {label}
        {#if modified}
          <span class="modified-pill">Unsaved</span>
        {/if}
        {#if locked}
          <span class="lock-inline" aria-hidden="true"><LockIcon /></span>
          <span class="sr-only">({checked ? "on" : "off"}, locked)</span>
        {/if}
      </span>
      <span id={hintId} class="option-hint" class:warning={hintTone === "warning"}>{hint}</span>
    </span>
    {#if locked}
      <span class="locked-switch" aria-hidden="true">
        <span {...attrs(ui.switch, checked && ui.checked)}><span {...attrs(ui.thumb, checked && ui.thumbChecked)}></span></span>
      </span>
    {:else}
      <Toggle {label} labelledBy={labelId} describedBy={hintId} bind:checked {onchange} />
    {/if}
  </span>
{/snippet}

{#if locked}
  <Tooltip text={lockedText} block onclick={onLockedClick ?? (() => {})}>
    <span
      class={`option-row locked ${flat ? "flat" : (attrs(ui.inset).class ?? "")}`}
      style={flat ? undefined : attrs(ui.inset).style}
    >
      {@render inner()}
    </span>
  </Tooltip>
{:else}
  <label
    class={`option-row interactive ${flat ? "flat" : (attrs(ui.inset).class ?? "")}`}
    style={flat ? undefined : attrs(ui.inset).style}
  >
    {@render inner()}
  </label>
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

  .option-row.interactive {
    cursor: pointer;
  }

  .option-row.flat {
    padding: 14px 0 0;
    border-top: 1px solid var(--browsify-border-strong);
  }

  .option-row.locked {
    cursor: help;
  }

  .locked-switch {
    display: flex;
    align-items: center;
    width: 44px;
    height: 44px;
    flex-shrink: 0;
    opacity: 0.5;
    pointer-events: none;
  }

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
    flex-wrap: wrap;
    gap: 6px;
  }

  .modified-pill {
    font-size: 12px;
    font-weight: 500;
    color: var(--log-warning);
    padding: 0 6px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--log-warning) 55%, var(--browsify-control-border));
    background: var(--browsify-panel);
  }

  .option-hint {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .option-hint.warning {
    color: var(--log-warning);
  }

  .lock-inline {
    display: inline-flex;
    color: var(--text-secondary);
  }

  .lock-inline :global(svg) {
    width: 12px;
    height: 12px;
  }
</style>
