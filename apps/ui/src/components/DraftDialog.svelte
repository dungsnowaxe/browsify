<script lang="ts">
  import { tick } from "svelte";
  import { activeDialog } from "../lib/draftDialog";

  let dialogEl: HTMLDialogElement | undefined = $state();
  let safeButton: HTMLButtonElement | undefined = $state();
  let restoreToMain = false;

  $effect(() => {
    const req = $activeDialog;
    if (!dialogEl) return;
    if (req) {
      if (!dialogEl.open) dialogEl.showModal();
      void tick().then(() => safeButton?.focus());
    } else if (dialogEl.open) {
      dialogEl.close();
      if (restoreToMain) {
        restoreToMain = false;
        void tick().then(() => document.getElementById("main-content")?.focus());
      }
    }
  });

  function cancel() {
    const req = $activeDialog;
    if (!req) return;
    if (req.kind === "discard" || req.kind === "confirmImport" || req.kind === "confirmClearLogs") {
      req.resolve(false);
    } else {
      req.resolve(null);
    }
  }

  function handleCancel(event: Event) {
    event.preventDefault();
    cancel();
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === dialogEl) cancel();
  }
</script>

<dialog
  bind:this={dialogEl}
  class="draft-dialog"
  aria-labelledby={$activeDialog ? "draft-dialog-title" : undefined}
  aria-describedby={$activeDialog ? "draft-dialog-desc" : undefined}
  oncancel={handleCancel}
  onclick={handleBackdropClick}
>
  {#if $activeDialog?.kind === "conflict"}
    {@const req = $activeDialog}
    <div class="dialog-body">
      <div class="dialog-head">
        <h2 id="draft-dialog-title">Resolve conflict in {req.field}</h2>
        {#if req.step}
          <span class="step-pill">
            Conflict {req.step}{#if req.total && req.total > 1} of {req.total}{/if}
          </span>
        {/if}
      </div>
      <p id="draft-dialog-desc" class="dialog-desc">
        {req.scope === "disk"
          ? "Your Primary Config changed outside this tab while you had unsaved edits. Choose which value to keep. Canceling keeps your draft open."
          : "Two open Browsify tabs edited the same setting. Choose which value to keep. Canceling keeps this tab’s edits open."}
      </p>

      <div class="compare-grid">
        <div class="compare-card base">
          <span class="compare-label">Original</span>
          <p class="compare-value">{req.base}</p>
        </div>
        <div class="compare-card">
          <span class="compare-label">{req.mineLabel}</span>
          <p class="compare-value">{req.mine}</p>
        </div>
        <div class="compare-card">
          <span class="compare-label">{req.theirsLabel}</span>
          <p class="compare-value">{req.theirs}</p>
        </div>
      </div>

      <div class="dialog-actions">
        <button bind:this={safeButton} type="button" class="app-button" onclick={cancel}>
          Cancel
        </button>
        <button type="button" class="app-button" onclick={() => req.resolve("theirs")}>
          Keep {req.theirsLabel.toLowerCase()}
        </button>
        <button type="button" class="app-button primary" onclick={() => req.resolve("mine")}>
          Keep {req.mineLabel.toLowerCase()}
        </button>
      </div>
    </div>
  {:else if $activeDialog?.kind === "identify"}
    {@const req = $activeDialog}
    <div class="dialog-body">
      <div class="dialog-head">
        <h2 id="draft-dialog-title">Match updated Handler Rule</h2>
        {#if req.step}
          <span class="step-pill">Step {req.step}</span>
        {/if}
      </div>
      <p id="draft-dialog-desc" class="dialog-desc">
        Your Primary Config now contains this rule. Choose which original Handler Rule it matches, or treat it as a new rule. Canceling keeps your draft open.
      </p>

      <div class="rule-preview">
        <span class="compare-label">Updated rule in Primary Config</span>
        <p class="compare-value">{req.rowSummary}</p>
      </div>

      <div class="candidate-list" role="group" aria-label="Original Handler Rules">
        {#each req.candidates as candidate (candidate.key)}
          <button
            type="button"
            class="candidate-option"
            onclick={() => req.resolve(candidate.key)}
          >
            <span class="candidate-badge">Rule {candidate.number}</span>
            <span class="candidate-summary">{candidate.summary}</span>
          </button>
        {/each}
        <button
          type="button"
          class="candidate-option new-option"
          onclick={() => req.resolve("new")}
        >
          <span class="candidate-badge">New</span>
          <span class="candidate-summary">Treat as a newly added Handler Rule</span>
        </button>
      </div>

      <div class="dialog-actions">
        <button bind:this={safeButton} type="button" class="app-button" onclick={cancel}>
          Cancel
        </button>
      </div>
    </div>
  {:else if $activeDialog?.kind === "discard"}
    {@const req = $activeDialog}
    <div class="dialog-body">
      <h2 id="draft-dialog-title">Discard unsaved changes?</h2>
      <p id="draft-dialog-desc" class="dialog-desc">
        {req.sharedAcrossTabs
          ? "This discards your unsaved edits across all open Browsify tabs and restores the active Primary Config."
          : "This discards your unsaved edits and restores the active Primary Config."}
      </p>

      <div class="dialog-actions">
        <button bind:this={safeButton} type="button" class="app-button" onclick={() => req.resolve(false)}>
          Keep editing
        </button>
        <button
          type="button"
          class="app-button danger-solid"
          onclick={() => {
            restoreToMain = true;
            req.resolve(true);
          }}
        >
          Discard changes
        </button>
      </div>
    </div>
  {:else if $activeDialog?.kind === "confirmImport"}
    {@const req = $activeDialog}
    <div class="dialog-body">
      <h2 id="draft-dialog-title">Replace active Primary Config?</h2>
      <p id="draft-dialog-desc" class="dialog-desc">
        Applying {req.sourceSummary} replaces your active Primary Config rules and preferences immediately and reloads Browsify.
      </p>

      <div class="dialog-actions">
        <button bind:this={safeButton} type="button" class="app-button" onclick={() => req.resolve(false)}>
          Cancel
        </button>
        <button
          type="button"
          class="app-button primary"
          onclick={() => {
            restoreToMain = true;
            req.resolve(true);
          }}
        >
          Use imported config
        </button>
      </div>
    </div>
  {:else if $activeDialog?.kind === "confirmClearLogs"}
    {@const req = $activeDialog}
    <div class="dialog-body">
      <h2 id="draft-dialog-title">Clear activity logs?</h2>
      <p id="draft-dialog-desc" class="dialog-desc">
        This clears {req.count} {req.count === 1 ? "entry" : "entries"} from this window and the running Browsify session.
      </p>

      <div class="dialog-actions">
        <button bind:this={safeButton} type="button" class="app-button" onclick={() => req.resolve(false)}>
          Keep logs
        </button>
        <button
          type="button"
          class="app-button danger-solid"
          onclick={() => {
            restoreToMain = true;
            req.resolve(true);
          }}
        >
          Clear logs
        </button>
      </div>
    </div>
  {/if}
</dialog>

<style>
  .draft-dialog {
    margin: auto;
    padding: 0;
    border: 1px solid var(--browsify-control-border);
    border-radius: var(--browsify-radius-panel);
    background: var(--browsify-panel);
    color: var(--text-primary);
    box-shadow: 0 24px 64px color-mix(in srgb, var(--browsify-rail) 45%, transparent);
    width: min(520px, calc(100vw - 32px));
    max-height: calc(100svh - 32px);
    overflow-y: auto;
  }

  .draft-dialog::backdrop {
    background: var(--browsify-backdrop);
  }

  @media (prefers-reduced-transparency: reduce) {
    .draft-dialog::backdrop {
      background: var(--browsify-rail);
    }
  }

  .dialog-body {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .dialog-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .step-pill {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--browsify-inset);
    border: 1px solid var(--browsify-border-strong);
    border-radius: 999px;
    padding: 3px 10px;
  }

  h2 {
    margin: 0;
    font: 500 20px/1.25 var(--font-display);
    letter-spacing: -0.025em;
    color: var(--text-primary);
  }

  .dialog-desc {
    margin: 0;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.5;
  }

  .compare-grid {
    display: grid;
    gap: 10px;
  }

  .compare-card,
  .rule-preview {
    padding: 12px 14px;
    border-radius: var(--browsify-radius-control);
    background: var(--browsify-inset);
    border: 1px solid var(--browsify-border-strong);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .compare-card.base {
    background: var(--browsify-surface);
  }

  .compare-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .compare-value {
    margin: 0;
    font-size: 14px;
    color: var(--text-primary);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .candidate-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .candidate-option {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    padding: 10px 14px;
    text-align: left;
    border-radius: var(--browsify-radius-control);
    border: 1px solid var(--browsify-control-border);
    background: var(--browsify-control);
    color: var(--text-primary);
    cursor: pointer;
    transition: background-color 150ms ease, border-color 150ms ease;
  }

  .candidate-option:hover {
    background: var(--browsify-inset);
    border-color: var(--browsify-link);
  }

  .candidate-badge {
    font-size: 12px;
    font-weight: 500;
    color: var(--browsify-link);
    flex-shrink: 0;
  }

  .candidate-summary {
    font-size: 14px;
    overflow-wrap: anywhere;
  }

  .dialog-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 4px;
  }
</style>
