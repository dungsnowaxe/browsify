<script lang="ts">
  import { Link } from "svelte-routing";
  import PreferencesIcon from "./icons/Preferences.svelte";
  import TestIcon from "./icons/Test.svelte";
  import LogsIcon from "./icons/Logs.svelte";
  import AboutIcon from "./icons/About.svelte";
  import RulesIcon from "./icons/Rules.svelte";
  import { nav } from "../design-system/navigation";
  import { attrs } from "../design-system/styles";

  export let numErrors = 0;
  export let onNavigate: () => void = () => {};

  const tabs = [
    { path: "/rules", label: "Rules", shortLabel: "Rules", component: RulesIcon },
    { path: "/test", label: "Test URL", shortLabel: "Test", component: TestIcon },
    { path: "/troubleshoot", label: "Activity", shortLabel: "Activity", showErrors: true, component: LogsIcon },
    { path: "/preferences", label: "Preferences", shortLabel: "Prefs", component: PreferencesIcon },
    { path: "/about", label: "About", shortLabel: "About", component: AboutIcon },
  ];

  function linkProps({ isCurrent }: { isCurrent: boolean }) {
    return isCurrent ? { "aria-current": "page" } : {};
  }
</script>

<aside {...attrs(nav.rail)}>
  <nav {...attrs(nav.list)} aria-label="Main navigation">
    <Link to="/" getProps={linkProps} on:click={onNavigate} let:active>
      <span class="home-link" class:active>
        <span {...attrs(nav.brand, active && nav.brandActive)}>
          <img src="/browsify-icon.svg" width="44" height="44" alt="" />
        </span>
        <span class="home-label">Home</span>
      </span>
    </Link>
    {#each tabs as tab, i}
      {#if i === 3}<div {...attrs(nav.spacer)}></div>{/if}
      <Link
        to={tab.path}
        getProps={linkProps}
        on:click={onNavigate}
        let:active
      >
        <span {...attrs(nav.item, active && nav.active)}>
          <span {...attrs(nav.icon, active && nav.activeIcon)} aria-hidden="true">
            <svelte:component this={tab.component} />
            {#if tab.showErrors && numErrors > 0}
              <span {...attrs(nav.badge)}>{numErrors}</span>
            {/if}
          </span>
          <span class="label-full">{tab.label}</span>
          <span class="label-short">{tab.shortLabel}</span>
          {#if tab.showErrors && numErrors > 0}
            <span class="sr-only">, {numErrors} {numErrors === 1 ? "error" : "errors"}</span>
          {/if}
        </span>
      </Link>
    {/each}
  </nav>
</aside>

<style>
  aside :global(a) { text-decoration: none; color: inherit; border-radius: 18px; min-width: 0; }
  aside :global(a:focus-visible) { outline: 2px solid #ffffff; outline-offset: 3px; }
  aside :global(svg) { width: 21px; height: 21px; }
  aside :global(img) { display: block; border-radius: 11px; width: 44px; height: 44px; }
  .label-short { display: none; }
  .home-link {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    margin-bottom: 12px;
  }
  .home-label {
    font-size: 12px;
    line-height: 1.15;
    color: var(--browsify-on-rail);
  }
  .home-link.active .home-label {
    color: #ffffff;
    font-weight: 500;
  }
  @media (max-width: 700px) {
    aside :global(a) {
      width: 100%;
      border-radius: 16px;
    }
    .label-full { display: none; }
    .label-short { display: inline; }
    .home-link {
      width: 100%;
      margin-bottom: 0;
      min-height: 48px;
      padding: 5px 2px;
      border-radius: 16px;
      gap: 3px;
    }
    .home-link.active {
      background: var(--browsify-rail-active-bg);
      box-shadow: inset 0 0 0 1px var(--browsify-rail-active-border), var(--browsify-shadow-icon);
    }
    .home-link.active .home-label {
      color: var(--browsify-rail-active-ink);
    }
    aside :global(img) {
      width: 22px;
      height: 22px;
      border-radius: 6px;
    }
    aside :global(svg) {
      width: 19px;
      height: 19px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    aside :global(a span) { transform: none !important; }
  }
</style>
