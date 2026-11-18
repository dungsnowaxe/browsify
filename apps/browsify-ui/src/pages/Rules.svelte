<script lang="ts">
  import { onMount, untrack } from "svelte";
  import PageContainer from "../components/PageContainer.svelte";
  import Select from "../components/Select.svelte";

  const fieldPrefix = $props.id();
  import type { Choice, EditorGroup, EditorRow, ProfileChoices, RuleEditor, RuleSource } from "../types";

  let { editor, editorError, ruleSaving, browserChoices, profileChoices }: {
    editor: RuleEditor | null;
    editorError: string;
    ruleSaving: RuleSource | null;
    browserChoices: Choice[] | null;
    profileChoices: Record<string, ProfileChoices>;
  } = $props();

  type DraftRow = EditorRow & { key: number; rule: (Omit<NonNullable<EditorRow["rule"]>, "match"> & { match: string[] }) | null };
  type Draft = { group: EditorGroup; rows: DraftRow[]; dirty: boolean };
  let drafts = $state<Draft[]>([]);
  let discard = false;
  let nextKey = 0;
  const requested = new Set<string>();
  const rpc = (type: string, data = {}) => window.finicky.sendMessage({ type, ...data });

  function toDraft(group: EditorGroup): Draft {
    return { group, dirty: false, rows: group.rows.map(row => ({
      ...row, key: nextKey++, rule: row.rule ? {
        ...row.rule, match: Array.isArray(row.rule.match) ? [...row.rule.match] : [row.rule.match ?? ""],
      } : null,
    })) };
  }
  $effect(() => {
    if (!editor) return;
    const { groups, saved } = editor;
    untrack(() => {
      drafts = groups.map(group => {
        const draft = drafts.find(d => d.group.source === group.source);
        return !discard && draft?.dirty && saved !== group.source ? draft : toDraft(group);
      });
      discard = false;
    });
  });

  function browserFor(id: string): Choice | undefined {
    return browserChoices?.find(choice => choice.id.toLowerCase() === id.toLowerCase() || choice.aliases.some(a => a.toLowerCase() === id.toLowerCase()));
  }
  function profileFor(id: string, choices: Choice[]): Choice | undefined {
    const exact = choices.find(choice => choice.id === id);
    if (exact) return exact;
    const matches = choices.filter(choice => [choice.id, ...choice.aliases].some(a => a.toLowerCase() === id.toLowerCase()));
    return matches.length === 1 ? matches[0] : undefined;
  }
  function refreshBrowsers() {
    rpc("getBrowserChoices");
    for (const browser of requested) rpc("getProfileChoices", { browser });
  }
  onMount(() => {
    rpc("getRuleEditor");
    refreshBrowsers();
    window.addEventListener("focus", refreshBrowsers);
    return () => window.removeEventListener("focus", refreshBrowsers);
  });
  $effect(() => {
    for (const draft of drafts) for (const { rule } of draft.rows) {
      if (!rule) continue;
      const browser = browserFor(rule.browser)?.id;
      if (browser && !requested.has(browser)) {
        requested.add(browser);
        rpc("getProfileChoices", { browser });
      }
    }
  });
  function add(draft: Draft) {
    draft.rows.push({ key: nextKey++, rule: { match: [""], browser: "", profile: "" } });
    draft.dirty = true;
  }
  function move(draft: Draft, from: number, by: number) {
    const [row] = draft.rows.splice(from, 1);
    draft.rows.splice(from + by, 0, row);
    draft.dirty = true;
  }
  function reload() {
    if (drafts.some(d => d.dirty) && !window.confirm("Discard unsaved changes and reload the rules from disk?")) return;
    discard = true;
    rpc("getRuleEditor");
  }
  function save(draft: Draft) {
    if (!draft.group.revision || ruleSaving) return;
    rpc("saveRuleEditor", { payload: {
      source: draft.group.source, revision: draft.group.revision,
      rows: draft.rows.map(({ id, rule }) => ({ id, rule })),
    } });
  }
</script>

<PageContainer eyebrow="Routing" title="Rules">
  {#snippet description()}Rewrites run first, then the first matching rule decides where the link opens. Use <code>*</code> as a wildcard, for example <code>*example.com/*</code>.{/snippet}

  <div class="actions toolbar">
    <button type="button" class="app-button" disabled={!!ruleSaving} onclick={reload}>Reload rules</button>
    <button type="button" class="app-button" onclick={refreshBrowsers}>Refresh browsers and profiles</button>
    {#if drafts.some(d => d.group.source === "primary")}
      <button type="button" class="app-button" onclick={() => rpc("editAdvancedRules")}>Open advanced rules in TextEdit</button>
    {/if}
  </div>
  {#if editorError}<p class="notice" role="alert">{editorError}</p>{/if}
  {#if !editor && !editorError}<p role="status">Loading rules…</p>{/if}

  {#each drafts as draft (draft.group.source)}
    <section class="group" aria-label={draft.group.source === "primary" ? "Advanced rules file" : "Rules added in Browsify"}>
      <header>
        <h2>{draft.group.source === "primary" ? "Advanced rules file" : "Rules added in Browsify"}</h2>
        {#if draft.group.source === "primary"}
          <p>Rules from this file take priority.</p>
        {/if}
      </header>
      {#if draft.group.rewrites?.length}
        <h3>Rewrites</h3>
        <ul class="read-only glass-panel">{#each draft.group.rewrites as rewrite}<li>{rewrite}</li>{/each}</ul>
      {/if}
      {#if !draft.rows.length}<p>No rules here yet.</p>{/if}
      {#each draft.rows as row, i (row.key)}
        {@const rule = row.rule}
        {#if !draft.group.revision || !rule}
          <div class="rule glass-panel read-row"><span>{i + 1}. {row.description ?? "Advanced rule"}</span><small>Edit in TextEdit</small></div>
        {:else}
          {@const browser = browserFor(rule.browser)}
          {@const catalog = browser ? profileChoices[browser.id] : undefined}
          {@const profile = profileFor(rule.profile ?? "", catalog?.profiles ?? [])}
          <fieldset class="rule glass-panel" disabled={!!ruleSaving}>
            <legend>Rule {i + 1}</legend>
            <div class="destinations">
              <div class="destination-field">
                <label for={`${fieldPrefix}-browser-${row.key}`}>Browser</label>
                <Select id={`${fieldPrefix}-browser-${row.key}`} value={browser?.id ?? rule.browser}
                  onchange={(event) => { rule.browser = event.currentTarget.value; rule.profile = ""; draft.dirty = true; }}>
                  <option value="" disabled>{browserChoices === null ? "Finding browsers…" : "Choose a browser"}</option>
                  {#if rule.browser && !browser}<option value={rule.browser}>{rule.browser === "none" ? "Don’t open" : `${rule.browser} (${browserChoices ? "unavailable" : "saved"})`}</option>{/if}
                  {#each browserChoices ?? [] as choice (choice.id)}<option value={choice.id}>{choice.label}</option>{/each}
                </Select>
              </div>
              {#if browser && (!catalog || catalog.error || catalog.profiles.length) || rule.profile}
                <div class="destination-field">
                  <label for={`${fieldPrefix}-profile-${row.key}`}>Profile</label>
                  <Select id={`${fieldPrefix}-profile-${row.key}`} value={profile?.id ?? rule.profile ?? ""} disabled={!!browser && (!catalog || !!catalog.error)}
                    aria-describedby={catalog?.error ? `${fieldPrefix}-profile-error-${row.key}` : undefined}
                    onchange={(event) => { rule.profile = event.currentTarget.value; draft.dirty = true; }}>
                    <option value="">{browser && !catalog ? "Finding profiles…" : "Use browser default"}</option>
                    {#if rule.profile && !profile}<option value={rule.profile}>{rule.profile} ({!browser || catalog && !catalog.error ? "unavailable" : "saved"})</option>{/if}
                    {#each catalog?.profiles ?? [] as choice (choice.id)}<option value={choice.id}>{choice.label}</option>{/each}
                  </Select>
                  {#if catalog?.error}<small id={`${fieldPrefix}-profile-error-${row.key}`}>Profiles couldn’t be read. Refresh to try again.</small>{/if}
                </div>
              {/if}
            </div>
            {#each rule.match as pattern, j}
              <div class="pattern">
                <label>URL pattern {rule.match.length > 1 ? j + 1 : ""}
                  <input class="app-input" value={pattern} placeholder="*example.com/*" oninput={event => { rule.match[j] = event.currentTarget.value; draft.dirty = true; }} />
                </label>
                {#if rule.match.length > 1}<button type="button" class="app-button" onclick={() => { rule.match.splice(j, 1); draft.dirty = true; }}>Remove pattern</button>{/if}
              </div>
            {/each}
            <div class="actions row-actions">
              <button type="button" class="app-button" onclick={() => { rule.match.push(""); draft.dirty = true; }}>Add pattern</button>
              <button type="button" class="app-button" disabled={i === 0} onclick={() => move(draft, i, -1)}>Move up</button>
              <button type="button" class="app-button" disabled={i === draft.rows.length - 1} onclick={() => move(draft, i, 1)}>Move down</button>
              <button type="button" class="app-button" onclick={() => { draft.rows.splice(i, 1); draft.dirty = true; }}>Delete rule</button>
            </div>
          </fieldset>
        {/if}
      {/each}
      {#if draft.group.revision}
        <div class="actions">
          <button type="button" class="app-button" disabled={!!ruleSaving} onclick={() => add(draft)}>Add rule</button>
          <button type="button" class="app-button save" disabled={!!ruleSaving || !draft.dirty} onclick={() => save(draft)}>{ruleSaving === draft.group.source ? "Saving…" : "Save rules"}</button>
          {#if draft.dirty}<small>Unsaved changes</small>{/if}
        </div>
      {/if}
    </section>
  {/each}
</PageContainer>

<style>
  .group { display: grid; gap: 14px; margin-block: 24px; }
  h2, h3, p { margin: 0; }
  h2 { font-size: 1.12rem; }
  h3 { font-size: .94rem; }
  p, small { color: var(--text-secondary); }
  header p { margin-top: 4px; }
  .rule, .read-only { min-width: 0; margin: 0; border-radius: var(--browsify-radius-panel); padding: 14px; }
  .read-only { padding-inline-start: 34px; overflow-wrap: anywhere; }
  .read-row { display: flex; justify-content: space-between; gap: 12px; flex-wrap: wrap; overflow-wrap: anywhere; }
  legend { padding-inline: 5px; color: var(--text-secondary); }
  .destinations { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(200px, 100%), 1fr)); gap: 12px; }
  .destination-field, label { display: grid; gap: 5px; min-width: 0; }
  .destination-field { align-content: start; }
  label { font-size: 13px; color: var(--text-secondary); }
  .app-input { min-width: 0; width: 100%; }
  .pattern { margin-top: 10px; display: flex; gap: 8px; align-items: end; }
  .pattern label { flex: 1; }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .row-actions { margin-top: 12px; }
  .notice { margin-top: 12px; color: var(--log-error); }
  .save:not(:disabled) { background: var(--browsify-accent); color: var(--browsify-on-accent); }
</style>
