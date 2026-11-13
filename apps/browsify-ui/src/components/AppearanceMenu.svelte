<script lang="ts">
  import { attrs, ui } from "../design-system/styles";
  import { appearance, type Appearance } from "../design-system/theme";

  const modes: Appearance[] = ["system", "light", "dark"];

  function label(mode: Appearance): string {
    return mode[0].toUpperCase() + mode.slice(1);
  }

  function onKeydown(event: KeyboardEvent, index: number) {
    let next: number;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") next = (index + 1) % modes.length;
    else if (event.key === "ArrowLeft" || event.key === "ArrowUp") next = (index + modes.length - 1) % modes.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = modes.length - 1;
    else return;
    event.preventDefault();
    appearance.set(modes[next]);
    const group = (event.currentTarget as HTMLButtonElement).parentElement;
    group?.querySelectorAll<HTMLButtonElement>("button")[next]?.focus();
  }
</script>

<div {...attrs(ui.appearanceGroup)} role="radiogroup" aria-label="Appearance">
  {#each modes as mode, index}
    <button
      type="button"
      role="radio"
      aria-checked={$appearance === mode}
      tabindex={$appearance === mode ? 0 : -1}
      onkeydown={(event) => onKeydown(event, index)}
      {...attrs(ui.appearanceOption, $appearance === mode && ui.appearanceActive)}
      onclick={() => appearance.set(mode)}
    >
      {label(mode)}
    </button>
  {/each}
</div>
