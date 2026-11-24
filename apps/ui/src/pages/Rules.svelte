<script lang="ts">
  import { onMount } from "svelte";
  import PageContainer from "../components/PageContainer.svelte";
  import Select from "../components/Select.svelte";
  import { draft, edit, refresh, saving } from "../lib/configDraft";
  import type { DraftRow } from "../lib/configMerge";
  import type { Choice, ProfileChoices, Rule } from "../types";
  let {browserChoices,profileChoices}: {browserChoices:Choice[]|null; profileChoices: Record<string, ProfileChoices>} = $props();
  const fieldPrefix = $props.id();
  const requested = new Set<string>();
  const rpc = (type:string,data={}) => window.finicky.sendMessage({type,...data});
  function update(row:DraftRow, change:(rule:Rule)=>void) {
    edit(v=>{const rule=v.rows.find(r=>r.key===row.key)?.rule;if(rule)change(rule);});
  }
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
    void refresh();
    refreshBrowsers();
    window.addEventListener("focus", refreshBrowsers);
    return () => window.removeEventListener("focus", refreshBrowsers);
  });
  $effect(() => {
    for (const { rule } of $draft?.current.rows ?? []) {
      if (!rule) continue;
      const browser = browserFor(rule.browser)?.id;
      if (browser && !requested.has(browser)) {
        requested.add(browser);
        rpc("getProfileChoices", { browser });
      }
    }
  });
  function add() {
    edit(v=>v.rows.push({key:crypto.randomUUID(),rule:{match:[""],browser:"",profile:""}}));
  }
  function move(from:number, by:number) {
    edit(v=>{const [row]=v.rows.splice(from,1);v.rows.splice(from+by,0,row);});
  }
</script>

<PageContainer eyebrow="Routing" title="Rules">
  {#snippet description()}Rewrites run first, then the first matching rule decides where the link opens. Use <code>*</code> as a wildcard, for example <code>*example.com/*</code>.{/snippet}

  <div class="actions toolbar">
    <button type="button" class="app-button" onclick={refreshBrowsers}>Refresh browsers and profiles</button>
    <button type="button" class="app-button" onclick={() => rpc("editAdvancedRules")}>Open TOML in TextEdit</button>
  </div>
  {#if $draft}
    <section class="group" aria-label="Handler rules">
      {#if $draft.base.rewrites?.length}
        <h3>Rewrites</h3>
        <ul class="read-only glass-panel">{#each $draft.base.rewrites as rewrite}<li>{rewrite}</li>{/each}</ul>
      {/if}
      {#if !$draft.current.rows.length}<p>No rules here yet.</p>{/if}
      {#each $draft.current.rows as row, i (row.key)}
        {@const rule = row.rule}
        {#if !$draft.base.editableRows || !rule}
          <div class="rule glass-panel read-row"><span>{i + 1}. {row.description ?? "Advanced rule"}</span><small>Edit in TextEdit</small></div>
        {:else}
          {@const browser = browserFor(rule.browser)}
          {@const catalog = browser ? profileChoices[browser.id] : undefined}
          {@const profile = profileFor(rule.profile ?? "", catalog?.profiles ?? [])}
          <fieldset class="rule glass-panel" disabled={$saving}>
            <legend>Rule {i + 1}</legend>
            <div class="destinations">
              <div class="destination-field">
                <label for={`${fieldPrefix}-browser-${row.key}`}>Browser</label>
                <Select id={`${fieldPrefix}-browser-${row.key}`} value={browser?.id ?? rule.browser}
                  onchange={(event) => { const browser=event.currentTarget.value; update(row,r=>{r.browser=browser;r.profile="";}); }}>
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
                    onchange={(event) => { const profile=event.currentTarget.value;update(row,r=>{r.profile=profile;}); }}>
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
                  <input class="app-input" value={pattern} placeholder="*example.com/*" oninput={event => { const pattern=event.currentTarget.value;update(row,r=>{r.match[j]=pattern;}); }} />
                </label>
                {#if rule.match.length > 1}<button type="button" class="app-button" onclick={() => { update(row,r=>r.match.splice(j,1)); }}>Remove pattern</button>{/if}
              </div>
            {/each}
            <div class="actions row-actions">
              <button type="button" class="app-button" onclick={() => { update(row,r=>r.match.push("")); }}>Add pattern</button>
              <button type="button" class="app-button" disabled={i === 0} onclick={() => move(i, -1)}>Move up</button>
              <button type="button" class="app-button" disabled={i === $draft.current.rows.length - 1} onclick={() => move(i, 1)}>Move down</button>
              <button type="button" class="app-button" onclick={() => { edit(v=>v.rows.splice(i,1)); }}>Delete rule</button>
            </div>
          </fieldset>
        {/if}
      {/each}
      {#if $draft.base.editableRows}<button type="button" class="app-button" disabled={$saving} onclick={add}>Add rule</button>{/if}
    </section>
  {:else}<p>Loading rules…</p>{/if}
</PageContainer>

<style>
  .group { display: grid; gap: 14px; margin-block: 24px; }
  h3, p { margin: 0; }
  h3 { font-size: .94rem; }
  p, small { color: var(--text-secondary); }
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
</style>
