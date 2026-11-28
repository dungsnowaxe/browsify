<script lang="ts">
  import type { LogEntry } from "../types";
  import { formatLogEntry } from "../utils/text";

  export let messageBuffer: LogEntry[] = [];
  export let showDebug = true;

  function levelBadge(level: string): string {
    switch (level.toLowerCase()) {
      case "error":
        return "ERR";
      case "warn":
      case "warning":
        return "WARN";
      case "debug":
        return "DBG";
      default:
        return "INFO";
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="log-content"
  role="log"
  aria-live="polite"
  aria-label="Activity log entries"
  tabindex="0"
>
  <ol class="log-list">
    {#each messageBuffer as entry}
      {#if showDebug || entry.level.toLowerCase() !== "debug"}
        {@const level = entry.level.toLowerCase()}
        <li class="log-entry">
          <span class="log-time" title={entry.time}>
            {new Date(entry.time).toLocaleTimeString()}
          </span>
          <span class="log-badge log-level-{level}">{levelBadge(level)}</span>
          <div class="log-message log-level-{level}">
            {#each formatLogEntry(entry) as part}
              {#if part.type === "url"}
                <a href={part.content} target="_blank" rel="noopener noreferrer"
                  >{part.content}</a
                >
              {:else}
                {part.content}
              {/if}
            {/each}
          </div>
        </li>
      {/if}
    {/each}
  </ol>
</div>

<style>
  .log-content {
    overflow: auto;
    font-family: var(--font-mono);
    line-height: 1.5;
    color: var(--text-primary);
    flex: 1;
    border-radius: 8px;
  }

  .log-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .log-entry {
    display: flex;
    justify-content: flex-start;
    gap: 1em;
    margin-bottom: 8px;
    align-items: flex-start;
  }

  .log-time {
    color: var(--text-secondary);
    white-space: nowrap;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .log-badge {
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.02em;
    padding: 0 6px;
    border-radius: 6px;
    border: 1px solid var(--browsify-control-border);
    background: var(--browsify-inset);
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .log-badge.log-level-error {
    color: var(--log-error);
    border-color: var(--log-error);
  }

  .log-badge.log-level-warn,
  .log-badge.log-level-warning {
    color: var(--log-warning);
    border-color: var(--log-warning);
  }

  .log-badge.log-level-debug {
    color: var(--browsify-link);
    border-color: var(--browsify-link);
  }

  .log-message {
    flex-grow: 1;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 13px;
  }

  .log-level-error {
    color: var(--log-error);
  }

  .log-level-warn,
  .log-level-warning {
    color: var(--log-warning);
  }

  .log-level-debug {
    color: var(--log-debug);
  }

  :global(.log-message a) {
    color: inherit;
    text-decoration: underline;
    text-decoration-style: dotted;
  }

  :global(.log-message a:hover) {
    text-decoration-style: solid;
  }
</style>
