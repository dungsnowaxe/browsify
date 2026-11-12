<script lang="ts">
  import XIcon from "./icons/X.svelte";
  const CUSTOM = "__custom__";
  const fieldId = `browser-${Math.random().toString(36).slice(2, 9)}`;

  let {
    browser, profile, isCustom, isProfileCustom,
    installedBrowsers = [], profilesByBrowser = {},
    disabled = false, required = false, hideProfile = false,
    browserPlaceholder = "Select browser",
    onBrowserChange, onProfileChange, onRequestProfiles, onSave, onInput,
  }: {
    browser: string;
    profile: string;
    isCustom: boolean;
    isProfileCustom: boolean;
    installedBrowsers: string[];
    profilesByBrowser: Record<string, string[]>;
    disabled?: boolean;
    required?: boolean;
    hideProfile?: boolean;
    browserPlaceholder?: string;
    onBrowserChange?: (browser: string, profile: string, isCustom: boolean) => void;
    onProfileChange?: (profile: string, isProfileCustom: boolean) => void;
    onRequestProfiles?: (browser: string) => void;
    onSave?: () => void;
    onInput?: () => void;
  } = $props();

  function handleBrowserSelect(event: Event) {
    const value = (event.target as HTMLSelectElement).value;
    if (value === CUSTOM) {
      onBrowserChange?.("", "", true);
    } else {
      if (value && profilesByBrowser[value] === undefined) onRequestProfiles?.(value);
      onBrowserChange?.(value, "", false);
      onSave?.();
    }
  }

  function handleProfileSelect(event: Event) {
    const value = (event.target as HTMLSelectElement).value;
    if (value === CUSTOM) onProfileChange?.("", true);
    else { onProfileChange?.(value, false); onSave?.(); }
  }
</script>

<div class="browser-select-row">
  <div class="field">
    <label for={fieldId}>Browser</label>
    <div class="field-control">
      {#if isCustom}
        <input id={fieldId} class="browser-input" type="text" placeholder="Browser name or bundle identifier" value={browser} oninput={(event) => { onBrowserChange?.(event.currentTarget.value, profile, true); onInput?.(); }} onblur={() => onSave?.()} {disabled} />
        <button type="button" class="clear-btn" {disabled} onclick={() => { onBrowserChange?.("", "", false); onSave?.(); }} aria-label="Choose an installed browser"><XIcon /></button>
      {:else}
        <div class="select-wrapper" class:empty={required && !browser}>
          <select id={fieldId} class="browser-dropdown" value={browser} onchange={handleBrowserSelect} {disabled}>
            <option value="">{browserPlaceholder}</option>
            {#each installedBrowsers as item}<option value={item}>{item}</option>{/each}
            <option value={CUSTOM}>Custom browser…</option>
          </select>
        </div>
      {/if}
    </div>
  </div>

  {#if !hideProfile && !isCustom && browser}
    <div class="field">
      <label for={`${fieldId}-profile`}>Profile</label>
      <div class="field-control">
        {#if isProfileCustom}
          <input id={`${fieldId}-profile`} class="browser-input" type="text" placeholder="Profile name" value={profile} oninput={(event) => { onProfileChange?.(event.currentTarget.value, true); onInput?.(); }} onblur={() => onSave?.()} {disabled} />
          <button type="button" class="clear-btn" {disabled} onclick={() => { onProfileChange?.("", false); onSave?.(); }} aria-label="Choose a browser profile"><XIcon /></button>
        {:else}
          <div class="select-wrapper">
            <select id={`${fieldId}-profile`} class="browser-dropdown" value={profile} onchange={handleProfileSelect} {disabled}>
              <option value="">Browser default</option>
              {#each profilesByBrowser[browser] ?? [] as item}<option value={item}>{item}</option>{/each}
              <option value={CUSTOM}>Custom profile…</option>
            </select>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .browser-select-row { width: 100%; min-width: 0; display: flex; gap: 16px; align-items: flex-start; }
  .field { flex: 1; min-width: 0; }
  label { display: block; font-size: 13px; color: var(--text-secondary); margin-bottom: 6px; }
  .field-control { display: flex; align-items: center; gap: 6px; }
  .select-wrapper { position: relative; flex: 1; min-width: 0; }
  .select-wrapper::after { content: ""; position: absolute; right: 16px; top: 50%; transform: translateY(-70%) rotate(45deg); width: 5px; height: 5px; border-right: 1.5px solid var(--text-secondary); border-bottom: 1.5px solid var(--text-secondary); pointer-events: none; }
  .browser-dropdown, .browser-input { width: 100%; min-height: 44px; padding: 10px 36px 10px 14px; background: var(--browsify-control); border: 1px solid var(--browsify-border-strong); border-radius: var(--browsify-radius-control); color: var(--text-primary); font-size: 14px; appearance: none; }
  .browser-input { flex: 1; min-width: 0; padding-right: 14px; }
  .browser-dropdown:focus-visible, .browser-input:focus-visible { outline: 2px solid var(--browsify-link); outline-offset: 2px; }
  .select-wrapper.empty .browser-dropdown { border-color: var(--log-warning); }
  .browser-dropdown:disabled, .browser-input:disabled { opacity: 0.65; cursor: default; }
  .select-wrapper:has(.browser-dropdown:disabled)::after { opacity: 0.5; }
  .clear-btn { background: var(--browsify-inset); border: 1px solid var(--browsify-border-strong); color: var(--text-secondary); cursor: pointer; width: 44px; height: 44px; padding: 12px; border-radius: 14px; flex-shrink: 0; display: flex; }
  .clear-btn:not(:disabled):hover { color: var(--log-error); }
  .clear-btn:disabled { opacity: 0.5; cursor: default; }
  @media (max-width: 800px) { .browser-select-row { flex-direction: column; gap: 12px; } .field { width: 100%; } }
</style>
