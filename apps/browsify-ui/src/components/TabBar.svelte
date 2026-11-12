<script lang="ts">
  import { Link } from "svelte-routing";
  import PreferencesIcon from "./icons/Preferences.svelte";
  import TestIcon from "./icons/Test.svelte";
  import LogsIcon from "./icons/Logs.svelte";
  import AboutIcon from "./icons/About.svelte";
  import RulesIcon from "./icons/Rules.svelte";
  import DashboardIcon from "./icons/Dashboard.svelte";
  import { nav } from "../design-system/navigation";
  import { attrs } from "../design-system/styles";

  export let numErrors = 0;
  export let onNavigate: () => void = () => {};

  const tabs = [
    { path: "/", label: "Overview", component: DashboardIcon },
    { path: "/rules", label: "Rules", component: RulesIcon },
    { path: "/test", label: "Test URL", component: TestIcon },
    { path: "/troubleshoot", label: "Activity", showErrors: true, component: LogsIcon },
    { path: "/preferences", label: "Settings", component: PreferencesIcon },
    { path: "/about", label: "About", component: AboutIcon },
  ];
</script>

<aside {...attrs(nav.rail)}>
  <Link to="/" aria-label="Browsify overview" on:click={onNavigate}>
    <span {...attrs(nav.brand)}>
      <svg width="25" height="25" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M5 18 18 5M8 5h10v10" />
        <path d="M5 11v7h7" />
      </svg>
    </span>
  </Link>
  <nav {...attrs(nav.list)} aria-label="Main navigation">
    {#each tabs as tab, i}
      {#if i === 4}<div {...attrs(nav.spacer)}></div>{/if}
      <Link to={tab.path} on:click={onNavigate} let:active>
        <span {...attrs(nav.item, active && nav.active)}>
          <span {...attrs(nav.icon, active && nav.activeIcon)} aria-hidden="true">
            <svelte:component this={tab.component} />
            {#if tab.showErrors && numErrors > 0}
              <span {...attrs(nav.badge)}>{numErrors}</span>
            {/if}
          </span>
          <span>{tab.label}{#if tab.showErrors && numErrors > 0}<span class="sr-only">, {numErrors} errors</span>{/if}</span>
        </span>
      </Link>
    {/each}
  </nav>
</aside>

<style>
  aside :global(a) { text-decoration: none; color: inherit; border-radius: 24px; }
  aside :global(svg) { width: 21px; height: 21px; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
  @media (prefers-reduced-motion: reduce) {
    aside :global(a span) { transform: none !important; }
  }
</style>
