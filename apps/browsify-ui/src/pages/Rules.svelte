<script lang="ts">
  import { untrack, tick, onMount, onDestroy } from "svelte";
  import PageContainer from "../components/PageContainer.svelte";
  import BrowserProfileSelector from "../components/BrowserProfileSelector.svelte";
  import WarningIcon from "../components/icons/Warning.svelte";
  import XIcon from "../components/icons/X.svelte";
  import type { Rule, RulesFile } from "../types";

  let {
    rulesFile = { defaultBrowser: "", rules: [] },
    installedBrowsers = [],
    profilesByBrowser = {},
    hasPrimaryConfig = false,
  }: {
    rulesFile: RulesFile;
    installedBrowsers: string[];
    profilesByBrowser: Record<string, string[]>;
    hasPrimaryConfig: boolean;
  } = $props();

  const SAVE_DEBOUNCE = 3000;

  // Local editable copies
  let rules = $state<Rule[]>([]);

  // Per-row "Custom..." state
  let rowIsCustom = $state<boolean[]>([]);
  let rowProfileIsCustom = $state<boolean[]>([]);

  function profileOptions(browser: string): string[] {
    return profilesByBrowser[browser] ?? [];
  }

  let saveTimer: ReturnType<typeof setTimeout>;
  let focusTarget = $state<{ rule: number; pattern: number } | null>(null);

  function patternNeedsWildcard(pattern: string): boolean {
    return !/\*/.test(pattern.trim()) && pattern.trim().length > 0;
  }

  function autofocusNew(node: HTMLInputElement, coords: { rule: number; pattern: number }) {
    if (focusTarget && focusTarget.rule === coords.rule && focusTarget.pattern === coords.pattern) {
      tick().then(() => { node.focus(); focusTarget = null; });
    }
  }
  let pendingSave = $state(false);

  function save() {
    clearTimeout(saveTimer);
    const payload: RulesFile = {
      defaultBrowser: rulesFile.defaultBrowser,
      defaultProfile: rulesFile.defaultProfile,
      options: rulesFile.options,
      rules,
    };
    window.finicky.sendMessage({ type: "saveRules", payload });
    pendingSave = false;
  }

  function scheduleSave() {
    clearTimeout(saveTimer);
    pendingSave = true;
    saveTimer = setTimeout(() => {
      const payload: RulesFile = {
        defaultBrowser: rulesFile.defaultBrowser,
        defaultProfile: rulesFile.defaultProfile,
        options: rulesFile.options,
        rules,
      };
      window.finicky.sendMessage({ type: "saveRules", payload });
      pendingSave = false;
    }, SAVE_DEBOUNCE);
  }

  onDestroy(() => {
    clearTimeout(saveTimer);
  });

  function onRowMatchInput(i: number, j: number, e: Event) {
    const newMatch = [...rules[i].match];
    newMatch[j] = (e.target as HTMLInputElement).value;
    rules[i] = { ...rules[i], match: newMatch };
    scheduleSave();
  }

  function addPattern(i: number) {
    const j = rules[i].match.length;
    rules[i] = { ...rules[i], match: [...rules[i].match, ""] };
    focusTarget = { rule: i, pattern: j };
  }

  function removePattern(i: number, j: number) {
    const newMatch = rules[i].match.filter((_, idx) => idx !== j);
    rules[i] = { ...rules[i], match: newMatch.length > 0 ? newMatch : [""] };
    save();
  }

  function addRule() {
    const i = rules.length;
    rules = [...rules, { match: [""], browser: "", profile: "" }];
    rowIsCustom = [...rowIsCustom, false];
    rowProfileIsCustom = [...rowProfileIsCustom, false];
    focusTarget = { rule: i, pattern: 0 };
  }

  function removeRule(i: number) {
    rules = rules.filter((_, idx) => idx !== i);
    rowIsCustom = rowIsCustom.filter((_, idx) => idx !== i);
    rowProfileIsCustom = rowProfileIsCustom.filter((_, idx) => idx !== i);
    save();
  }

  // Drag-to-reorder
  let dragIndex = $state<number | null>(null);

  function onDragStart(i: number) {
    dragIndex = i;
  }

  function onDragOver(e: DragEvent, i: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === i) return;
    const newRules = [...rules];
    const newCustom = [...rowIsCustom];
    const newProfileCustom = [...rowProfileIsCustom];
    const [movedRule] = newRules.splice(dragIndex, 1);
    const [movedCustom] = newCustom.splice(dragIndex, 1);
    const [movedProfileCustom] = newProfileCustom.splice(dragIndex, 1);
    newRules.splice(i, 0, movedRule);
    newCustom.splice(i, 0, movedCustom);
    newProfileCustom.splice(i, 0, movedProfileCustom);
    rules = newRules;
    rowIsCustom = newCustom;
    rowProfileIsCustom = newProfileCustom;
    dragIndex = i;
  }

  function onDragEnd() {
    dragIndex = null;
    scheduleSave();
  }

  // Request data from native on mount
  onMount(() => {
    window.finicky.sendMessage({ type: "getRules" });
    window.finicky.sendMessage({ type: "getInstalledBrowsers" });
  });

  onDestroy(() => clearTimeout(saveTimer));

  // Sync incoming props into local state when they change; also fetch profiles for known browsers
  $effect(() => {
    if (pendingSave) return;
    const newRules = rulesFile.rules.map((r: Rule) => ({
      ...r,
      match: Array.isArray(r.match) ? r.match : r.match ? [r.match as unknown as string] : [""],
    }));
    rules = newRules;
    rowIsCustom = newRules.map(
      (r) => r.browser !== "" && !installedBrowsers.includes(r.browser)
    );
    rowProfileIsCustom = newRules.map(
      (r) => {
        const profile = r.profile ?? "";
        if (profile === "") return false;
        const profiles = untrack(() => profilesByBrowser[r.browser]);
        if (profiles === undefined) return false; // not fetched yet — correct later
        return !profiles.includes(profile);
      }
    );
    // Fetch profiles for browsers not yet loaded — untrack to avoid re-triggering on profilesByBrowser changes
    const browsersToFetch = new Set<string>();
    for (const r of newRules) {
      if (r.browser) browsersToFetch.add(r.browser);
    }
    untrack(() => {
      for (const b of browsersToFetch) {
        if (profilesByBrowser[b] === undefined) {
          window.finicky.sendMessage({ type: "getBrowserProfiles", browser: b });
        }
      }
    });
  });

  // Once profile lists arrive, correct any rowProfileIsCustom entries that were
  // deferred because profilesByBrowser wasn't loaded yet during the initial sync.
  $effect(() => {
    rules.forEach((r, i) => {
      const profile = r.profile ?? "";
      if (profile === "") return;
      const profiles = profilesByBrowser[r.browser];
      if (profiles === undefined) return; // still loading
      rowProfileIsCustom[i] = !profiles.includes(profile);
    });
  });
</script>

<PageContainer eyebrow="Routing" title="Handler rules">
  {#snippet description()}
    {#if hasPrimaryConfig}
      The first matching rule wins. <span style="color: var(--accent-color)">Primary Config is active</span> — its handlers run first and take priority over these rules.
    {:else}
      The first matching rule wins. Use <code>*</code> as a wildcard, e.g. <code>*example.com/*</code>.
    {/if}
  {/snippet}
  <!-- Rules list -->
  {#if rules.length === 0}
    <div class="empty-rules">
      No rules yet. Add one below.
    </div>
  {:else}
    <div class="rules-list" role="list">
      {#each rules as rule, i}
          <div
            class="rule-row"
            class:dragging={dragIndex === i}
            class:no-browser={!rule.browser && !rowIsCustom[i]}
            draggable="true"
            ondragstart={() => onDragStart(i)}
            ondragover={(e) => onDragOver(e, i)}
            ondragend={onDragEnd}
            role="listitem"
          >
            <div class="rule-top">
              <div class="drag-handle" aria-hidden="true"><svg width="14" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="9" cy="5" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="9" cy="19" r="1"/><circle cx="15" cy="5" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="15" cy="19" r="1"/></svg></div>
              <BrowserProfileSelector
                browser={rule.browser}
                profile={rule.profile ?? ""}
                isCustom={rowIsCustom[i]}
                isProfileCustom={rowProfileIsCustom[i]}
                {installedBrowsers}
                {profilesByBrowser}
                required
                browserPlaceholder="Select browser"
                onBrowserChange={(browser, profile, isCustom) => {
                  rules[i] = { ...rules[i], browser, profile };
                  rowIsCustom[i] = isCustom;
                  rowProfileIsCustom[i] = false;
                }}
                onProfileChange={(profile, isProfileCustom) => {
                  rules[i] = { ...rules[i], profile };
                  rowProfileIsCustom[i] = isProfileCustom;
                }}
                onRequestProfiles={(b) => window.finicky.sendMessage({ type: "getBrowserProfiles", browser: b })}
                onSave={save}
                onInput={scheduleSave}
              />
              {#if !rule.browser && !rowIsCustom[i]}
                <span class="browser-required-hint">
                  <WarningIcon />
                  Browser required
                </span>
              {/if}
              <button
                class="delete-btn"
                onclick={() => removeRule(i)}
                aria-label="Delete rule"
              >
                <XIcon />
              </button>
            </div>

            <div class="rule-bottom">
              <div class="patterns">
                {#each rule.match as pattern, j}
                  <div class="pattern-row">
                    <div class="pattern-input-wrapper" class:has-warning={patternNeedsWildcard(pattern)}>
                      <label class="pattern-label" for={`rule-${i}-pattern-${j}`}>URL pattern {j + 1}</label>
                      <input
                        id={`rule-${i}-pattern-${j}`}
                        class="text-input pattern-input"
                        type="text"
                        placeholder="*.example.com/*"
                        value={pattern}
                        oninput={(e) => onRowMatchInput(i, j, e)}
                        onblur={() => save()}
                        use:autofocusNew={{ rule: i, pattern: j }}
                      />
                      {#if patternNeedsWildcard(pattern)}
                        <span class="pattern-warning-icon" aria-label="Exact URLs rarely match"><WarningIcon /></span>
                      {/if}
                    </div>
                    {#if rule.match.length > 1}
                      <button
                        class="remove-pattern-btn"
                        onclick={() => removePattern(i, j)}
                        aria-label="Remove pattern"
                      ><XIcon /></button>
                    {/if}
                  </div>
                {/each}
                <button class="add-pattern-btn" onclick={() => addPattern(i)}>+ Add pattern</button>
              </div>
            </div>
          </div>
        {/each}
    </div>
  {/if}

  <button class="add-rule-btn" onclick={addRule}>+ Add rule</button>
</PageContainer>

<style>
  .text-input {
    padding: 10px 14px;
    background: var(--browsify-control);
    border: 1px solid var(--browsify-border);
    border-radius: var(--browsify-radius-control);
    color: var(--text-primary);
    font-size: 0.9em;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    box-shadow: var(--browsify-shadow-control);
  }

  .text-input:focus {
    outline: none;
    border-color: var(--accent-color);
  }

  .text-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.4;
  }

  .empty-rules {
    color: var(--text-secondary);
    font-size: 0.95em;
    padding: 28px 20px;
    text-align: center;
    background: var(--browsify-panel);
    background-image: var(--browsify-panel-gradient);
    backdrop-filter: blur(22px) saturate(1.4);
    border: 1px solid var(--browsify-border);
    border-radius: var(--browsify-radius-panel);
    box-shadow: var(--browsify-shadow);
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .rule-row {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px 18px;
    background: var(--browsify-panel);
    background-image: var(--browsify-panel-gradient);
    backdrop-filter: blur(22px) saturate(1.4);
    border-radius: var(--browsify-radius-panel);
    border: 1px solid var(--browsify-border);
    box-shadow: var(--browsify-shadow);
    transition: background 0.15s, transform 0.15s;
  }

  .rule-row:hover {
    background: var(--bg-hover);
  }

  .rule-row.no-browser {
    border-left: 3px solid var(--log-warning);
    padding-left: 9px;
  }

  .browser-required-hint {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--log-warning);
    font-size: 0.75em;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .browser-required-hint :global(svg) {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .rule-row.dragging {
    outline: 2px solid var(--accent-color);
    outline-offset: -2px;
    opacity: 0.7;
  }

  .rule-top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .rule-bottom {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
    padding-left: 22px;
  }

  .drag-handle {
    color: var(--text-secondary);
    opacity: 0.4;
    cursor: grab;
    font-size: 1.1em;
    user-select: none;
    flex-shrink: 0;
  }

  .patterns {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }

  .pattern-row {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .pattern-input-wrapper {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .pattern-label { display: block; font-size: 13px; color: var(--text-secondary); margin-bottom: 6px; }

  .pattern-input-wrapper.has-warning .pattern-input {
    padding-right: 32px;
  }

  .pattern-warning-icon {
    position: absolute;
    right: 7px;
    top: calc(50% + 12px);
    transform: translateY(-50%);
    display: flex;
    color: var(--log-warning);
    line-height: 1;
    cursor: help;
  }

  .pattern-warning-icon::after {
    content: "Exact URLs rarely match, maybe you want use a wildcard?";
    display: none;
    position: absolute;
    bottom: calc(100% + 6px);
    right: 0;
    background: var(--card-bg, #222);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
    font-size: 0.78rem;
    font-family: inherit;
    white-space: nowrap;
    padding: 4px 8px;
    border-radius: 5px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    pointer-events: none;
    z-index: 20;
  }

  .pattern-warning-icon:hover::after {
    display: block;
  }

  .pattern-input {
    width: 100%;
  }

  .remove-pattern-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 3px;
    border-radius: 4px;
    flex-shrink: 0;
    display: flex;
    transition: color 0.15s;
  }

  .remove-pattern-btn:hover {
    color: var(--log-error);
  }

  .add-pattern-btn {
    align-self: flex-start;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 13px;
    min-height: 32px;
    padding: 4px 0;
    cursor: pointer;
    transition: color 0.15s;
  }

  .add-pattern-btn:hover {
    color: var(--accent-color);
  }

  .delete-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    border-radius: 50%;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    margin-left: auto;
    transition: color 0.15s;
  }

  .delete-btn:hover {
    color: var(--log-error);
  }

  .add-rule-btn {
    align-self: flex-start;
    background: var(--browsify-accent);
    border: none;
    border-radius: 999px;
    color: var(--browsify-on-accent);
    font-size: 0.9em;
    font-weight: 500;
    min-height: 44px;
    padding: 12px 18px;
    cursor: pointer;
    box-shadow: var(--browsify-shadow-control);
    transition: transform 0.15s ease, opacity 0.15s ease;
  }

  .add-rule-btn:hover {
    opacity: 0.92;
    transform: translateY(-1px);
  }

  .add-rule-btn:active { transform: translateY(1px) scale(0.985); }

  @media (max-width: 800px) {
    .browser-required-hint { display: none; }
    .rule-top { align-items: flex-start; }
    .rule-row { padding: 16px; }
    .rule-bottom { padding-left: 0; }
    .drag-handle { display: none; }
    .delete-btn { width: 32px; height: 44px; }
    .pattern-warning-icon::after { white-space: normal; width: 170px; }
  }
</style>
