<script>
  import Nav from './lib/Nav.svelte'
  import Dashboard from './routes/Dashboard.svelte'
  import SearchTerms from './routes/SearchTerms.svelte'
  import Sources from './routes/Sources.svelte'
  import Notifications from './routes/Notifications.svelte'

  let route = window.location.hash.slice(1) || '/'

  window.addEventListener('hashchange', () => {
    route = window.location.hash.slice(1) || '/'
  })
</script>

<div class="app">
  <Nav />
  <main>
    {#if route === '/'}
      <Dashboard />
    {:else if route === '/search-terms'}
      <SearchTerms />
    {:else if route === '/sources'}
      <Sources />
    {:else if route === '/notifications'}
      <Notifications />
    {/if}
  </main>
</div>

<style>
  :global(body) { margin: 0; font-family: system-ui, sans-serif; background: #f9f9f9; }
  .app { display: flex; min-height: 100vh; }
  main { flex: 1; padding: 1.5rem; }
</style>
