<script lang="ts">
  import Select from "./Select.svelte";
  import XIcon from "./icons/X.svelte";
  import type { Choice, ProfileChoices } from "../types";

  const CUSTOM = "__custom__";
  const SUPPRESS = "none";
  const fieldPrefix = $props.id();

  let {
    browser = "",
    profile = "",
    browserChoices = null,
    profileChoices = {},
    installedBrowsers = [],
    profilesByBrowser = {},
    disabled = false,
    required = false,
    suppressEmptyWarning = false,
    hideProfile = false,
    allowSuppress = false,
    allowCustom = false,
    browserPlaceholder = "Choose a browser",
    onBrowserChange,
    onProfileChange,
    onRequestProfiles,
  }: {
    browser: string;
    profile?: string;
    browserChoices?: Choice[] | null;
    profileChoices?: Record<string, ProfileChoices>;
    installedBrowsers?: string[];
    profilesByBrowser?: Record<string, string[]>;
    disabled?: boolean;
    required?: boolean;
    suppressEmptyWarning?: boolean;
    hideProfile?: boolean;
    allowSuppress?: boolean;
    allowCustom?: boolean;
    browserPlaceholder?: string;
    onBrowserChange: (browser: string, profile: string) => void;
    onProfileChange?: (profile: string) => void;
    onRequestProfiles?: (browser: string) => void;
  } = $props();

  let customBrowserMode = $state(false);
  let customProfileMode = $state(false);

  const resolvedChoices = $derived.by<Choice[] | null>(() => {
    if (browserChoices !== null) return browserChoices;
    if (installedBrowsers.length) {
      return installedBrowsers.map((name) => ({ id: name, label: name, aliases: [] }));
    }
    return null;
  });

  function findBrowserChoice(value: string, list: Choice[] | null): Choice | undefined {
    if (!value || !list) return undefined;
    const lower = value.toLowerCase();
    return list.find(
      (choice) =>
        choice.id.toLowerCase() === lower ||
        choice.aliases.some((alias) => alias.toLowerCase() === lower),
    );
  }

  function findProfileChoice(value: string, list: Choice[]): Choice | undefined {
    if (!value) return undefined;
    const exact = list.find((choice) => choice.id === value);
    if (exact) return exact;
    const lower = value.toLowerCase();
    const matches = list.filter((choice) =>
      [choice.id, ...choice.aliases].some((alias) => alias.toLowerCase() === lower),
    );
    return matches.length === 1 ? matches[0] : undefined;
  }

  const matchedBrowser = $derived(findBrowserChoice(browser, resolvedChoices));
  const isSuppressed = $derived(browser === SUPPRESS);
  const browserUnavailable = $derived(
    !!browser && !isSuppressed && resolvedChoices !== null && !matchedBrowser && !customBrowserMode,
  );

  const catalog = $derived.by<ProfileChoices | undefined>(() => {
    const key = matchedBrowser?.id ?? browser;
    if (!key || isSuppressed) return undefined;
    if (profileChoices[key]) return profileChoices[key];
    const fallback = profilesByBrowser[key];
    if (fallback !== undefined) {
      return { profiles: fallback.map((name) => ({ id: name, label: name, aliases: [] })) };
    }
    return undefined;
  });

  const matchedProfile = $derived(findProfileChoice(profile, catalog?.profiles ?? []));
  const profileUnavailable = $derived(
    !!profile &&
      !isSuppressed &&
      !customProfileMode &&
      (!matchedBrowser || (!!catalog && !catalog.error && !matchedProfile)),
  );

  const showProfileField = $derived(
    !hideProfile &&
      !isSuppressed &&
      (!!profile ||
        customProfileMode ||
        (!!matchedBrowser && (!catalog || !!catalog.error || catalog.profiles.length > 0 || allowCustom))),
  );

  const isDia = $derived.by(() => {
    const id = (matchedBrowser?.id ?? browser).toLowerCase();
    return id === "dia" || id === "company.thebrowser.dia";
  });

  const isZen = $derived.by(() => {
    const id = (matchedBrowser?.id ?? browser).toLowerCase();
    return id === "zen" || id === "app.zen-browser.zen";
  });

  $effect(() => {
    const key = matchedBrowser?.id;
    if (key && !hideProfile && !catalog) {
      onRequestProfiles?.(key);
    }
  });

  function handleBrowserSelect(event: Event) {
    const next = (event.currentTarget as HTMLSelectElement).value;
    if (next === CUSTOM) {
      customBrowserMode = true;
      customProfileMode = false;
      onBrowserChange("", "");
      return;
    }
    customBrowserMode = false;
    customProfileMode = false;
    onBrowserChange(next, "");
    if (next && next !== SUPPRESS) {
      onRequestProfiles?.(next);
    }
  }

  function handleProfileSelect(event: Event) {
    const next = (event.currentTarget as HTMLSelectElement).value;
    if (next === CUSTOM) {
      customProfileMode = true;
      onProfileChange?.("");
      return;
    }
    customProfileMode = false;
    onProfileChange?.(next);
  }
</script>

<div class="target-picker">
  <div class="picker-row">
    <div class="picker-field">
      <label for={`${fieldPrefix}-browser`}>Browser Target</label>
      <div class="field-control">
        {#if customBrowserMode}
          <input
            id={`${fieldPrefix}-browser`}
            class="app-input"
            type="text"
            placeholder="Browser name or bundle identifier"
            value={browser}
            {disabled}
            oninput={(event) => onBrowserChange(event.currentTarget.value, profile)}
          />
          <button
            type="button"
            class="clear-btn"
            {disabled}
            aria-label="Choose an installed browser"
            onclick={() => {
              customBrowserMode = false;
              onBrowserChange("", "");
            }}
          >
            <XIcon />
          </button>
        {:else}
          <Select
            id={`${fieldPrefix}-browser`}
            value={matchedBrowser?.id ?? browser}
            {disabled}
            warning={(required && !browser && !suppressEmptyWarning) || browserUnavailable}
            aria-describedby={browserUnavailable ? `${fieldPrefix}-browser-warn` : undefined}
            onchange={handleBrowserSelect}
          >
            <option value="" disabled={required}>
              {resolvedChoices === null ? "Finding browsers…" : browserPlaceholder}
            </option>
            {#if allowSuppress}
              <option value={SUPPRESS}>Don’t open</option>
            {/if}
            {#if browser && !isSuppressed && !matchedBrowser}
              <option value={browser}>
                {browser} ({resolvedChoices !== null ? "unavailable" : "saved"})
              </option>
            {/if}
            {#each resolvedChoices ?? [] as choice (choice.id)}
              <option value={choice.id}>{choice.label}</option>
            {/each}
            {#if allowCustom}
              <option value={CUSTOM}>Custom browser…</option>
            {/if}
          </Select>
        {/if}
      </div>
      {#if browserUnavailable}
        <p id={`${fieldPrefix}-browser-warn`} class="picker-note warning" role="status">
          Unavailable Destination: “{browser}” wasn’t found on this Mac. Browsify won’t silently fall back to another browser — choose an available browser or reinstall it.
        </p>
      {:else if customBrowserMode && browser && resolvedChoices !== null && !matchedBrowser}
        <p class="picker-note">
          Custom Browser Target. Make sure this browser name or bundle identifier is installed on this Mac.
        </p>
      {/if}
    </div>

    {#if showProfileField}
      <div class="picker-field">
        <label for={`${fieldPrefix}-profile`}>Profile</label>
        <div class="field-control">
          {#if customProfileMode}
            <input
              id={`${fieldPrefix}-profile`}
              class="app-input"
              type="text"
              placeholder="Profile or workspace name"
              value={profile}
              {disabled}
              oninput={(event) => onProfileChange?.(event.currentTarget.value)}
            />
            <button
              type="button"
              class="clear-btn"
              {disabled}
              aria-label="Choose a discovered profile"
              onclick={() => {
                customProfileMode = false;
                onProfileChange?.("");
              }}
            >
              <XIcon />
            </button>
          {:else}
            <Select
              id={`${fieldPrefix}-profile`}
              value={matchedProfile?.id ?? profile}
              disabled={disabled || (!!matchedBrowser && (!catalog || !!catalog.error))}
              warning={profileUnavailable}
              aria-describedby={catalog?.error
                ? `${fieldPrefix}-profile-error`
                : profileUnavailable
                  ? `${fieldPrefix}-profile-warn`
                  : undefined}
              onchange={handleProfileSelect}
            >
              <option value="">
                {matchedBrowser && !catalog ? "Finding profiles…" : "Use browser default"}
              </option>
              {#if profile && !matchedProfile}
                <option value={profile}>
                  {profile} ({!matchedBrowser || (catalog && !catalog.error) ? "unavailable" : "saved"})
                </option>
              {/if}
              {#each catalog?.profiles ?? [] as choice (choice.id)}
                <option value={choice.id}>{choice.label}</option>
              {/each}
              {#if allowCustom}
                <option value={CUSTOM}>Custom profile…</option>
              {/if}
            </Select>
          {/if}
        </div>
        {#if catalog?.error}
          <p id={`${fieldPrefix}-profile-error`} class="picker-note warning" role="status">
            Profiles couldn’t be read. Refresh browsers and profiles to try again.
          </p>
        {:else if profileUnavailable}
          <p id={`${fieldPrefix}-profile-warn`} class="picker-note warning" role="status">
            Unavailable Destination: profile “{profile}” wasn’t found for {matchedBrowser?.label ?? browser}. Browsify won’t silently open another profile.
          </p>
        {/if}
      </div>
    {/if}
  </div>

  {#if isDia}
    <p class="picker-note boundary">
      Dia requires an open window and macOS Automation permission. Use an exact, unique profile name; profiles share browsing data if enabled in Dia.
    </p>
  {/if}
  {#if isZen}
    <p class="picker-note boundary">
      Zen profiles are workspaces (spaces). Open Zen once so its workspaces appear. A missing or ambiguous workspace suppresses launch instead of falling back, and workspaces do not isolate cookies on their own.
    </p>
  {/if}
</div>

<style>
  .target-picker {
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .picker-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(220px, 100%), 1fr));
    gap: 14px;
    align-items: start;
  }

  .picker-field {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  label {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .field-control {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .clear-btn {
    background: var(--browsify-inset);
    border: 1px solid var(--browsify-control-border);
    color: var(--text-secondary);
    cursor: pointer;
    width: 44px;
    height: 44px;
    border-radius: var(--browsify-radius-control);
    flex-shrink: 0;
    display: grid;
    place-items: center;
  }

  .clear-btn:not(:disabled):hover {
    color: var(--log-error);
    border-color: var(--log-error);
  }

  .clear-btn:disabled {
    opacity: 0.5;
    border-color: var(--browsify-border-strong);
    cursor: default;
  }

  .picker-note {
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--text-secondary);
  }

  .picker-note.warning,
  .picker-note.boundary {
    color: var(--log-warning);
  }
</style>
