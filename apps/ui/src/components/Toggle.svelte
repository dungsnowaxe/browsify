<script lang="ts">
  import { attrs, ui } from "../design-system/styles";

  let {
    checked = $bindable(false),
    disabled = false,
    label,
    labelledBy,
    describedBy,
    onchange,
  }: {
    checked?: boolean;
    disabled?: boolean;
    label: string;
    labelledBy?: string;
    describedBy?: string;
    onchange?: () => void;
  } = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.();
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={labelledBy ? undefined : label}
  aria-labelledby={labelledBy}
  aria-describedby={describedBy}
  {disabled}
  class="switch-target"
  onclick={toggle}
>
  <span {...attrs(ui.switch, checked && ui.checked)}>
    <span {...attrs(ui.thumb, checked && ui.thumbChecked)}></span>
  </span>
</button>

<style>
  .switch-target { display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; width: 44px; height: 44px; border: 0; padding: 0; background: transparent; cursor: pointer; border-radius: 10px; }
  .switch-target:disabled { opacity: 0.5; cursor: default; }
  .switch-target:disabled > span { pointer-events: none; }
</style>
