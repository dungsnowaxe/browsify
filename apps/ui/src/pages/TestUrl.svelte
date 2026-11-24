<script lang="ts">
  import PageContainer from "../components/PageContainer.svelte";
  import UrlTester from "../components/UrlTester.svelte";
</script>

<PageContainer eyebrow="Browser targets" title="URL tester">
  {#snippet description()}
    Check the browser, profile and final URL your configuration selects. Nothing opens during a test.
  {/snippet}
  <UrlTester />
</PageContainer>
