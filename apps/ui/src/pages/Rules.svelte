<script lang="ts">
  import { onMount, tick } from "svelte";
  import PageContainer from "../components/PageContainer.svelte";
  import BrowserTargetPicker from "../components/BrowserTargetPicker.svelte";
  import {
    consumePendingRuleFocus,
    draft,
    draftLoading,
    edit,
    pendingRuleFocus,
    refresh,
    saving,
    validationRequested,
  } from "../lib/configDraft";
  import type { DraftRow } from "../lib/configMerge";
  import { toast } from "../lib/toast";
  import type { Choice, ProfileChoices, Rule } from "../types";

  let {
    browserChoices,
    profileChoices,
    installedBrowsers = [],
    profilesByBrowser = {},
    connectionStatus = "connecting",
    onRetry = () => {},
  }: {
    browserChoices: Choice[] | null;
    profileChoices: Record<string, ProfileChoices>;
    installedBrowsers?: string[];
    profilesByBrowser?: Record<string, string[]>;
    connectionStatus?: "connecting" | "connected" | "disconnected";
    onRetry?: () => void;
  } = $props();

  const fieldPrefix = $props.id();
  const requested = new Set<string>();
  const rpc = (type: string, data = {}) => window.finicky.sendMessage({ type, ...data });

  let addButton: HTMLButtonElement | undefined = $state();
  let rulesSection: HTMLElement | undefined = $state();
  let freshKeys = $state<string[]>([]);
  let collapsedKeys = $state<string[]>([]);
  let initializedCollapse = false;
  let lastValidationCount = 0;

  const editableRows = $derived(
    ($draft?.current.rows ?? []).filter((r) => !!r.rule),
  );
  const allCollapsed = $derived(
    editableRows.length > 0 && editableRows.every((r) => collapsedKeys.includes(r.key)),
  );

  function toggleCollapseAll() {
    if (allCollapsed) {
      collapsedKeys = [];
    } else {
      collapsedKeys = editableRows.map((r) => r.key);
    }
  }

  function markTouched(key: string) {
    if (freshKeys.includes(key)) {
      freshKeys = freshKeys.filter((k) => k !== key);
    }
  }

  function isCollapsed(key: string): boolean {
    return collapsedKeys.includes(key);
  }

  function toggleCollapsed(key: string) {
    if (collapsedKeys.includes(key)) {
      collapsedKeys = collapsedKeys.filter((k) => k !== key);
    } else {
      collapsedKeys = [...collapsedKeys, key];
    }
  }

  function expandRule(key: string) {
    if (collapsedKeys.includes(key)) {
      collapsedKeys = collapsedKeys.filter((k) => k !== key);
    }
  }

  function ruleSummary(rule: Rule): string {
    const patterns = rule.match.map((p) => p.trim()).filter(Boolean);
    const patText =
      patterns.length === 0
        ? "No pattern"
        : patterns.length === 1
          ? patterns[0]
          : `${patterns[0]} (+${patterns.length - 1})`;
    const targetText =
      rule.browser === "none"
        ? "Don’t open"
        : rule.browser.trim()
          ? `${rule.browser}${rule.profile ? ` (${rule.profile})` : ""}`
          : "No target";
    return `${patText} → ${targetText}`;
  }

  function update(row: DraftRow, change: (rule: Rule) => void) {
    edit((v) => {
      const rule = v.rows.find((r) => r.key === row.key)?.rule;
      if (rule) change(rule);
    });
  }

  function requestProfiles(browser: string) {
    if (!browser || requested.has(browser)) return;
    requested.add(browser);
    rpc("getProfileChoices", { browser });
  }

  function refreshBrowsers(notify = false) {
    rpc("getBrowserChoices");
    for (const browser of requested) rpc("getProfileChoices", { browser });
    if (notify) toast.info("Refreshing browsers and profiles…", 2200);
  }

  onMount(() => {
    void refresh();
    refreshBrowsers(false);
    const onFocus = () => refreshBrowsers(false);
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  });

  $effect(() => {
    if (!initializedCollapse && $draft) {
      initializedCollapse = true;
      if ($draft.current.rows.length > 3) {
        collapsedKeys = $draft.current.rows
          .filter((r) => r.rule && r.rule.browser.trim() && r.rule.match.some((p) => p.trim()))
          .map((r) => r.key);
      }
    }
  });

  $effect(() => {
    const count = $validationRequested;
    if (count > lastValidationCount) {
      lastValidationCount = count;
      freshKeys = [];
    }
  });

  $effect(() => {
    const req = $pendingRuleFocus;
    if (!req || !$draft) return;
    consumePendingRuleFocus();
    if (req.kind === "invalid") {
      freshKeys = [];
      const invalidRow = $draft.current.rows.find(
        (r) => r.rule && (!r.rule.browser.trim() || r.rule.match.every((p) => !p.trim())),
      );
      if (invalidRow) {
        expandRule(invalidRow.key);
        void tick().then(() => {
          const card = rulesSection?.querySelector<HTMLElement>(`[data-rule-key="${invalidRow.key}"]`);
          if (!card) return;
          const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
          card.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "nearest" });
          const missingBrowser = !invalidRow.rule?.browser.trim();
          const target = missingBrowser
            ? card.querySelector<HTMLElement>("select")
            : card.querySelector<HTMLElement>("input.app-input");
          (target ?? card.querySelector<HTMLElement>(".collapse-toggle"))?.focus();
        });
      }
    } else if (req.kind === "index") {
      const targetRow = $draft.current.rows[req.index - 1];
      if (targetRow) {
        expandRule(targetRow.key);
        void tick().then(() => {
          const card = rulesSection?.querySelector<HTMLElement>(`[data-rule-key="${targetRow.key}"]`);
          if (!card) return;
          const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
          card.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "nearest" });
          (card.querySelector<HTMLElement>(".collapse-toggle") ?? card.querySelector<HTMLElement>("button"))?.focus();
        });
      }
    }
  });

  async function add() {
    const newKey = crypto.randomUUID();
    freshKeys = [...freshKeys, newKey];
    edit((v) =>
      v.rows.push({
        key: newKey,
        rule: { match: [""], browser: "", profile: "" },
      }),
    );
    await tick();
    const card = rulesSection?.querySelector<HTMLElement>(`[data-rule-key="${newKey}"]`);
    if (card) {
      const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      card.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "nearest" });
      card.querySelector<HTMLElement>("input, select")?.focus();
    }
  }

  async function move(from: number, by: number, action: "up" | "down") {
    const row = $draft?.current.rows[from];
    if (!row) return;
    const key = row.key;
    edit((v) => {
      const [item] = v.rows.splice(from, 1);
      v.rows.splice(from + by, 0, item);
    });
    await tick();
    const card = rulesSection?.querySelector<HTMLElement>(`[data-rule-key="${key}"]`);
    const nextBtn = card?.querySelector<HTMLButtonElement>(`[data-order-action="${action}"]:not(:disabled)`);
    (nextBtn ?? card?.querySelector<HTMLElement>(".collapse-toggle"))?.focus();
  }

  async function moveToTop(from: number) {
    if (from <= 0) return;
    const row = $draft?.current.rows[from];
    if (!row) return;
    const key = row.key;
    edit((v) => {
      const [item] = v.rows.splice(from, 1);
      v.rows.unshift(item);
    });
    toast.info("Moved rule to position 1", 2400);
    await tick();
    const card = rulesSection?.querySelector<HTMLElement>(`[data-rule-key="${key}"]`);
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    card?.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "nearest" });
    card?.querySelector<HTMLElement>(".collapse-toggle")?.focus();
  }

  function handleRuleKeydown(event: KeyboardEvent, index: number) {
    if (!event.altKey || $saving || !$draft) return;
    const activeTag = document.activeElement?.tagName;
    if (activeTag === "INPUT" || activeTag === "SELECT" || activeTag === "TEXTAREA") return;
    if (event.key === "ArrowUp" && index > 0) {
      event.preventDefault();
      void move(index, -1, "up");
    } else if (event.key === "ArrowDown" && index < $draft.current.rows.length - 1) {
      event.preventDefault();
      void move(index, 1, "down");
    }
  }

  async function removePattern(row: DraftRow, patternIndex: number) {
    update(row, (r) => r.match.splice(patternIndex, 1));
    await tick();
    const nextIndex = Math.max(0, patternIndex - 1);
    const input = document.getElementById(`${fieldPrefix}-pattern-${row.key}-${nextIndex}`);
    input?.focus();
  }

  async function deleteRule(index: number, row: DraftRow) {
    const snapshot = structuredClone(row);
    freshKeys = freshKeys.filter((k) => k !== row.key);
    collapsedKeys = collapsedKeys.filter((k) => k !== row.key);
    edit((v) => {
      v.rows.splice(index, 1);
    });
    await tick();
    const cards = rulesSection?.querySelectorAll<HTMLElement>("[data-rule-key]");
    if (cards && cards.length > 0) {
      const target = cards[Math.min(index, cards.length - 1)];
      target?.querySelector<HTMLElement>(".collapse-toggle, button, select, input")?.focus();
    } else {
      addButton?.focus();
    }
    toast.info(`Handler Rule ${index + 1} deleted`, 6000, {
      label: "Undo",
      onClick: () => {
        edit((v) => {
          const insertAt = Math.min(index, v.rows.length);
          v.rows.splice(insertAt, 0, snapshot);
        });
      },
    });
  }
</script>

<PageContainer title="Rules">
  {#snippet description()}
    Rewrite Rules run first, then the first matching Handler Rule chooses the Browser Target. Use <code>*</code> as a wildcard, for example <code>*example.com/*</code>.
  {/snippet}

  {#if $draft}
    <div class="actions toolbar">
      {#if $draft.base.editableRows}
        <button bind:this={addButton} type="button" class="app-button primary" disabled={$saving} onclick={add}>
          Add rule
        </button>
      {/if}
      <button
        type="button"
        class="app-button"
        disabled={connectionStatus !== "connected"}
        onclick={() => refreshBrowsers(true)}
      >
        Refresh browsers and profiles
      </button>
      <button
        type="button"
        class="app-button"
        title="Open your Primary Config TOML file in TextEdit"
        disabled={connectionStatus !== "connected"}
        onclick={() => rpc("editAdvancedRules")}
      >
        Edit Primary Config
      </button>
      {#if editableRows.length > 1}
        <button type="button" class="app-button" onclick={toggleCollapseAll}>
          {allCollapsed ? "Expand all" : "Collapse all"}
        </button>
      {/if}
    </div>

    <p class="config-note">
      <strong>Edit Primary Config</strong> opens your TOML file in TextEdit for Rewrite Rules and Opener-scoped Handler Rules.
    </p>

    <details class="pattern-help">
      <summary>
        <svg class="disclosure-chevron" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="m6 4 4 4-4 4" />
        </svg>
        Pattern examples
      </summary>
      <ul>
        <li><code>*github.com/*</code> — matches any path or subdomain on github.com</li>
        <li><code>https://mail.google.com/*</code> — matches only Gmail over HTTPS</li>
        <li><code>*://*.client.example/*</code> — matches any subdomain of client.example</li>
      </ul>
    </details>

    {#if $draft.base.rewrites?.length}
      <section class="group" aria-labelledby="rewrites-heading">
        <h2 id="rewrites-heading">Rewrite Rules</h2>
        <ul class="read-only glass-panel">
          {#each $draft.base.rewrites as rewrite}
            <li>{rewrite}</li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="group" aria-labelledby="handlers-heading" bind:this={rulesSection}>
      <h2 id="handlers-heading">Handler Rules</h2>
      {#if !$draft.current.rows.length}
        <p class="empty-state">No Handler Rules yet. Add one to route specific links to a Browser Target.</p>
      {/if}
      {#each $draft.current.rows as row, i (row.key)}
        {@const rule = row.rule}
        {#if !$draft.base.editableRows || !rule}
          <article class="rule glass-panel read-row" data-rule-key={row.key}>
            <span>{i + 1}. {row.description ?? "Advanced Handler Rule"}</span>
            <button
              type="button"
              class="app-button"
              onclick={() => rpc("editAdvancedRules")}
            >
              Edit in Primary Config
            </button>
          </article>
        {:else}
          {@const missingPattern = rule.match.every((p) => !p.trim())}
          {@const missingBrowser = !rule.browser.trim()}
          {@const isFresh = freshKeys.includes(row.key)}
          {@const showValidation = !isFresh && (missingBrowser || missingPattern)}
          {@const collapsed = isCollapsed(row.key)}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <article
            class="rule glass-panel"
            class:collapsed
            data-rule-key={row.key}
            aria-labelledby={`${fieldPrefix}-rule-${row.key}`}
            onkeydown={(event) => handleRuleKeydown(event, i)}
          >
            <div class="rule-header">
              <h3 id={`${fieldPrefix}-rule-${row.key}`} class="rule-title-group">
                <button
                  type="button"
                  class="collapse-toggle"
                  aria-expanded={!collapsed}
                  onclick={() => toggleCollapsed(row.key)}
                >
                  <svg class="disclosure-chevron" class:open={!collapsed} viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="m6 4 4 4-4 4" />
                  </svg>
                  <span class="rule-number">Rule {i + 1}</span>
                  <span class="rule-summary" class:incomplete={showValidation}>
                    {ruleSummary(rule)}
                  </span>
                </button>
              </h3>
              <div class="rule-order">
                {#if i > 1}
                  <button
                    type="button"
                    class="ghost-action"
                    data-order-action="top"
                    aria-label={`Move Rule ${i + 1} to top`}
                    title="Move to top"
                    disabled={$saving}
                    onclick={() => moveToTop(i)}
                  >
                    Top
                  </button>
                {/if}
                <button
                  type="button"
                  class="ghost-action icon-only"
                  data-order-action="up"
                  aria-label={`Move Rule ${i + 1} up (Alt+Up)`}
                  title="Move up (Alt+↑)"
                  disabled={$saving || i === 0}
                  onclick={() => move(i, -1, "up")}
                >
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="M8 13V3M4 7l4-4 4 4" />
                  </svg>
                </button>
                <button
                  type="button"
                  class="ghost-action icon-only"
                  data-order-action="down"
                  aria-label={`Move Rule ${i + 1} down (Alt+Down)`}
                  title="Move down (Alt+↓)"
                  disabled={$saving || i === $draft.current.rows.length - 1}
                  onclick={() => move(i, 1, "down")}
                >
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="M8 3v10M4 9l4 4 4-4" />
                  </svg>
                </button>
              </div>
            </div>

            {#if !collapsed}
              <div class="patterns">
                {#each rule.match as pattern, j}
                  <div class="pattern">
                    <label for={`${fieldPrefix}-pattern-${row.key}-${j}`}>
                      URL pattern {rule.match.length > 1 ? j + 1 : ""}
                    </label>
                    <div class="pattern-row">
                      <input
                        id={`${fieldPrefix}-pattern-${row.key}-${j}`}
                        class="app-input"
                        class:invalid={!isFresh && missingPattern && j === 0}
                        type="text"
                        value={pattern}
                        placeholder="*example.com/*"
                        disabled={$saving}
                        onblur={() => markTouched(row.key)}
                        oninput={(event) => {
                          const value = event.currentTarget.value;
                          if (value.trim()) markTouched(row.key);
                          update(row, (r) => {
                            r.match[j] = value;
                          });
                        }}
                      />
                      {#if rule.match.length > 1}
                        <button
                          type="button"
                          class="ghost-action"
                          aria-label={`Remove URL pattern ${j + 1} from Rule ${i + 1}`}
                          disabled={$saving}
                          onclick={() => removePattern(row, j)}
                        >
                          Remove
                        </button>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>

              <BrowserTargetPicker
                browser={rule.browser}
                profile={rule.profile ?? ""}
                {browserChoices}
                {profileChoices}
                {installedBrowsers}
                {profilesByBrowser}
                disabled={$saving}
                required={true}
                suppressEmptyWarning={isFresh}
                allowSuppress={true}
                allowCustom={true}
                onRequestProfiles={requestProfiles}
                onBrowserChange={(browser, profile) => {
                  markTouched(row.key);
                  update(row, (r) => {
                    r.browser = browser;
                    r.profile = profile;
                  });
                }}
                onProfileChange={(profile) => {
                  update(row, (r) => {
                    r.profile = profile;
                  });
                }}
              />

              {#if showValidation}
                <p class="rule-validation" role="status">
                  {missingBrowser && missingPattern
                    ? "Enter at least one URL pattern and choose a Browser Target before saving."
                    : missingPattern
                      ? "Enter at least one URL pattern before saving."
                      : "Choose a Browser Target before saving."}
                </p>
              {/if}

              <div class="actions row-actions">
                <button
                  type="button"
                  class="app-button"
                  aria-label={`Add pattern to Rule ${i + 1}`}
                  disabled={$saving}
                  onclick={() => {
                    update(row, (r) => r.match.push(""));
                  }}
                >
                  Add pattern
                </button>
                <button
                  type="button"
                  class="ghost-action danger"
                  aria-label={`Delete Rule ${i + 1}`}
                  disabled={$saving}
                  onclick={() => deleteRule(i, row)}
                >
                  Delete rule
                </button>
              </div>
            {/if}
          </article>
        {/if}
      {/each}
    </section>
  {:else if $draftLoading && connectionStatus === "connecting"}
    <p class="empty-state">Loading rules…</p>
  {:else}
    <section class="disconnected-card glass-panel" aria-labelledby="rules-disconnected-heading">
      <h2 id="rules-disconnected-heading">Browsify is not connected</h2>
      <p class="empty-state">
        Open Browsify from Applications, Spotlight, or run <code>browsify ui</code>, then try again to load your Handler Rules.
      </p>
      <button type="button" class="app-button primary" onclick={onRetry}>
        Retry connection
      </button>
    </section>
  {/if}
</PageContainer>

<style>
  .group {
    display: grid;
    gap: 14px;
  }

  .config-note {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .config-note strong {
    font-weight: 500;
    color: var(--text-primary);
  }

  .pattern-help {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .pattern-help summary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    cursor: pointer;
    color: var(--browsify-link);
    font-weight: 500;
    list-style: none;
  }

  .pattern-help summary::-webkit-details-marker {
    display: none;
  }

  .disclosure-chevron {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    transition: transform 150ms ease;
  }

  .pattern-help[open] .disclosure-chevron,
  .disclosure-chevron.open {
    transform: rotate(90deg);
  }

  .pattern-help ul {
    margin: 4px 0 0;
    padding-inline-start: 20px;
    display: grid;
    gap: 6px;
  }

  .pattern-help code {
    font-family: var(--font-mono);
    background: var(--inset-bg);
    padding: 1px 5px;
    border-radius: 4px;
    color: var(--text-primary);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    flex-shrink: 0;
  }

  .empty-state {
    margin: 0;
    color: var(--text-secondary);
  }

  .disconnected-card {
    padding: 20px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .disconnected-card code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    background: var(--browsify-inset);
    padding: 1px 5px;
    border-radius: 4px;
  }

  .rule,
  .read-only {
    min-width: 0;
    margin: 0;
    border-radius: var(--browsify-radius-panel);
    padding: 18px;
  }

  .rule.collapsed {
    padding: 10px 16px;
  }

  .rule {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .rule-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .rule-title-group {
    flex: 1 1 200px;
    min-width: 0;
    margin: 0;
    font-size: 14px;
    font-weight: 500;
  }

  .rule-number {
    color: var(--text-primary);
    font-weight: 500;
    flex-shrink: 0;
  }

  .collapse-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 100%;
    min-height: 44px;
    padding: 4px 8px 4px 0;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    text-align: left;
    cursor: pointer;
    border-radius: 10px;
  }

  .collapse-toggle:hover {
    color: var(--text-primary);
  }

  .rule-summary {
    font-size: 13px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rule-summary.incomplete {
    color: var(--log-warning);
  }

  .rule-order {
    display: flex;
    gap: 4px;
    align-items: center;
    flex-shrink: 0;
  }

  .ghost-action {
    min-height: 44px;
    min-width: 44px;
    padding: 0 10px;
    border: 1px solid transparent;
    border-radius: 12px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 140ms ease, color 140ms ease, border-color 140ms ease;
  }

  .ghost-action.icon-only {
    padding: 0;
    width: 44px;
  }

  .ghost-action svg {
    width: 16px;
    height: 16px;
  }

  .ghost-action:not(:disabled):hover {
    background: var(--browsify-inset);
    color: var(--text-primary);
    border-color: var(--browsify-border-strong);
  }

  .ghost-action.danger {
    color: var(--log-error);
  }

  .ghost-action.danger:not(:disabled):hover {
    background: color-mix(in srgb, var(--log-error) 6%, var(--browsify-control));
    color: var(--log-error);
    border-color: color-mix(in srgb, var(--log-error) 35%, var(--browsify-border-strong));
  }

  .ghost-action:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .read-only {
    padding-inline-start: 34px;
    overflow-wrap: anywhere;
  }

  .read-row {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    overflow-wrap: anywhere;
  }

  .patterns {
    display: grid;
    gap: 10px;
  }

  .pattern {
    display: grid;
    gap: 6px;
  }

  .pattern label {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .pattern-row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .pattern-row .app-input {
    flex: 1 1 220px;
  }

  .rule-validation {
    margin: 0;
    font-size: 13px;
    color: var(--log-warning);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .row-actions {
    justify-content: space-between;
    padding-top: 4px;
  }
</style>
