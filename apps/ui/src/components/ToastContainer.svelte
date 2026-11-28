<script lang="ts">
  import { toast } from "../lib/toast";
  import type { Toast } from "../lib/toast";
  import { fade, fly } from "svelte/transition";
  import { onDestroy, onMount } from "svelte";
  import { flip } from "svelte/animate";
  import XIcon from "./icons/X.svelte";

  export let offsetLevel: 0 | 1 | 2 = 0;

  let toasts: Toast[] = [];
  let reducedMotion = false;

  const unsubscribe = toast.subscribe((value) => {
    toasts = value;
  });

  onMount(() => {
    if (typeof window === "undefined") return;
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    reducedMotion = media.matches;
    const onChange = (event: MediaQueryListEvent) => {
      reducedMotion = event.matches;
    };
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  });

  onDestroy(unsubscribe);
</script>

<div
  class="toast-container"
  class:elevated={offsetLevel === 1}
  class:double-elevated={offsetLevel === 2}
>
  {#each toasts as t (t.id)}
    <div
      class="toast toast-{t.type}"
      class:paused={t.paused}
      role={t.type === "error" ? "alert" : "status"}
      aria-live={t.type === "error" ? "assertive" : "polite"}
      on:mouseenter={() => toast.pause(t.id)}
      on:mouseleave={() => toast.resume(t.id)}
      on:focusin={() => toast.pause(t.id)}
      on:focusout={() => toast.resume(t.id)}
      in:fly={{ y: reducedMotion ? 0 : 24, duration: reducedMotion ? 0 : 180 }}
      out:fade={{ duration: reducedMotion ? 0 : 160 }}
      animate:flip={{ duration: reducedMotion ? 0 : 240 }}
    >
      <div class="toast-content">
        <span class="toast-icon" aria-hidden="true">
          {#if t.type === "success"}
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="10" cy="10" r="8" />
              <path d="m6.8 10.2 2.2 2.2 4.2-4.6" />
            </svg>
          {:else if t.type === "error"}
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="10" cy="10" r="8" />
              <path d="M10 6.5v4.2m0 2.8h.01" />
            </svg>
          {:else if t.type === "warning"}
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M10 3.2 17.5 16.5H2.5L10 3.2Z" />
              <path d="M10 8.2v3.5m0 2.5h.01" />
            </svg>
          {:else}
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="10" cy="10" r="8" />
              <path d="M10 9v4.5m0-7h.01" />
            </svg>
          {/if}
        </span>
        <span class="toast-message">{t.message}</span>
        {#if t.action}
          <button
            type="button"
            class="toast-action"
            on:click={() => {
              t.action?.onClick();
              toast.remove(t.id);
            }}
          >
            {t.action.label}
          </button>
        {/if}
        <button
          type="button"
          class="toast-close"
          aria-label="Dismiss notification"
          on:click={() => toast.remove(t.id)}
        >
          <XIcon />
        </button>
      </div>
      {#if t.extra}
        <pre class="toast-extra">{t.extra}</pre>
      {/if}
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: absolute;
    bottom: 76px;
    right: 28px;
    max-width: calc(100% - 48px);
    z-index: 9999;
    display: flex;
    flex-direction: column-reverse;
    gap: 12px;
    pointer-events: none;
  }

  .toast-container.elevated {
    bottom: 138px;
  }

  .toast-container.double-elevated {
    bottom: 196px;
  }

  .toast {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    padding: 8px 10px 8px 14px;
    border-radius: 20px;
    background: var(--browsify-panel);
    background-image: var(--browsify-panel-gradient);
    border: 1px solid var(--browsify-border-strong);
    box-shadow: var(--browsify-shadow);
    min-width: 0;
    width: 400px;
    max-width: 100%;
    pointer-events: auto;
    position: relative;
  }

  @media (max-width: 700px) {
    .toast-container {
      bottom: 64px;
      right: 12px;
      left: 12px;
      max-width: none;
    }

    .toast-container.elevated {
      bottom: 124px;
    }

    .toast-container.double-elevated {
      bottom: 184px;
    }

    .toast {
      width: 100%;
    }
  }

  .toast-content {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: 44px;
  }

  .toast-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
  }

  .toast-icon svg {
    width: 18px;
    height: 18px;
  }

  .toast-message {
    flex: 1;
    font-size: 13px;
    font-weight: 500;
    line-height: 1.4;
  }

  .toast-action {
    flex-shrink: 0;
    min-height: 44px;
    padding: 0 12px;
    border: 1px solid var(--browsify-control-border);
    border-radius: 999px;
    background: var(--browsify-control);
    color: var(--browsify-link);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 140ms ease;
  }

  .toast-action:hover {
    background: var(--browsify-inset);
  }

  .toast-close {
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background-color 140ms ease, color 140ms ease;
  }

  .toast-close:hover {
    background: var(--browsify-inset);
    color: var(--text-primary);
  }

  .toast-extra {
    margin: 0 4px 4px 0;
    font-family: var(--font-mono);
    background: var(--browsify-inset);
    border: 1px solid var(--browsify-border-strong);
    color: var(--text-primary);
    padding: 8px 10px;
    border-radius: 10px;
    font-size: 12px;
    line-height: 1.45;
    overflow-x: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .toast-success {
    color: var(--log-success);
  }

  .toast-error {
    color: var(--log-error);
    border-color: color-mix(in srgb, var(--log-error) 45%, var(--browsify-border-strong));
  }

  .toast-warning {
    color: var(--log-warning);
  }

  .toast-info {
    color: var(--text-primary);
  }
</style>
