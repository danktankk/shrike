<script>
  import Nav from './lib/Nav.svelte'
  import Dashboard from './routes/Dashboard.svelte'
  import SearchTerms from './routes/SearchTerms.svelte'
  import Sources from './routes/Sources.svelte'
  import Notifications from './routes/Notifications.svelte'

  let route = window.location.hash.slice(1) || '/'
  window.addEventListener('hashchange', () => { route = window.location.hash.slice(1) || '/' })
</script>

<div class="app">
  <Nav {route} />
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
  :global(*, *::before, *::after) { box-sizing: border-box; }

  :global(:root) {
    --bg:         #07070f;
    --surface:    #0e0e1c;
    --surface-2:  #131324;
    --surface-3:  #1a1a30;
    --border:     #1f1f38;
    --border-2:   #2a2a48;
    --accent:     #f97316;
    --accent-dim: rgba(249, 115, 22, 0.12);
    --accent-glow:rgba(249, 115, 22, 0.25);
    --text:       #e2e2f0;
    --text-muted: #5a5a80;
    --text-dim:   #3a3a60;
    --green:      #22c55e;
    --green-dim:  rgba(34, 197, 94, 0.12);
    --red:        #ef4444;
    --red-dim:    rgba(239, 68, 68, 0.12);
    --blue:       #60a5fa;
    --blue-dim:   rgba(96, 165, 250, 0.12);
    --font-head:  'Syne', sans-serif;
    --font-body:  'Outfit', sans-serif;
    --font-mono:  'JetBrains Mono', monospace;
  }

  :global(body) {
    margin: 0;
    font-family: var(--font-body);
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
    line-height: 1.5;
    -webkit-font-smoothing: antialiased;
  }

  :global(h1, h2, h3) {
    font-family: var(--font-head);
    margin: 0;
  }

  :global(a) { color: var(--accent); text-decoration: none; }
  :global(a:hover) { text-decoration: underline; }

  :global(code) {
    font-family: var(--font-mono);
    font-size: 0.85em;
    background: var(--surface-3);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--accent);
  }

  :global(.page) {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  :global(.page-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 1.25rem;
    border-bottom: 1px solid var(--border);
  }

  :global(.page-title) {
    font-family: var(--font-head);
    font-size: 1.6rem;
    font-weight: 800;
    color: var(--text);
    letter-spacing: -0.02em;
  }

  :global(.btn) {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 1rem;
    border-radius: 6px;
    border: 1px solid var(--border-2);
    background: var(--surface-2);
    color: var(--text);
    font-family: var(--font-body);
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }
  :global(.btn:hover) {
    border-color: var(--border-2);
    background: var(--surface-3);
    color: var(--text);
  }

  :global(.btn-primary) {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  :global(.btn-primary:hover) {
    background: #ea6a0a;
    border-color: #ea6a0a;
    color: #fff;
  }

  :global(.btn-danger) {
    background: transparent;
    border-color: transparent;
    color: var(--red);
  }
  :global(.btn-danger:hover) {
    background: var(--red-dim);
    border-color: var(--red);
  }

  :global(.btn-ghost) {
    background: transparent;
    border-color: transparent;
    color: var(--text-muted);
  }
  :global(.btn-ghost:hover) {
    background: var(--surface-3);
    color: var(--text);
    border-color: var(--border);
  }

  :global(.data-table) {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }
  :global(.data-table th) {
    text-align: left;
    padding: 0.6rem 0.875rem;
    font-family: var(--font-body);
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  :global(.data-table td) {
    padding: 0.7rem 0.875rem;
    border-bottom: 1px solid var(--border);
    color: var(--text);
    vertical-align: middle;
  }
  :global(.data-table tbody tr:hover td) {
    background: var(--surface-2);
  }
  :global(.data-table tbody tr:last-child td) {
    border-bottom: none;
  }

  :global(.table-wrap) {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }

  :global(.badge) {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 20px;
    font-size: 0.72rem;
    font-weight: 600;
    font-family: var(--font-mono);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  :global(.badge-rss)     { background: var(--green-dim); color: var(--green); }
  :global(.badge-newznab) { background: var(--blue-dim);  color: var(--blue); }
  :global(.badge-torznab)  { background: var(--accent-dim); color: var(--accent); }
  :global(.badge-prowlarr) { background: var(--accent-dim); color: var(--accent); }

  :global(.status-dot) {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    margin-right: 6px;
  }
  :global(.status-dot.on)  { background: var(--green); box-shadow: 0 0 6px var(--green); }
  :global(.status-dot.off) { background: var(--text-dim); }

  :global(.error-msg) {
    background: var(--red-dim);
    border: 1px solid var(--red);
    color: var(--red);
    padding: 0.6rem 1rem;
    border-radius: 6px;
    font-size: 0.875rem;
  }

  :global(.empty-state) {
    padding: 3rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  :global(.actions-cell) {
    display: flex;
    gap: 0.2rem;
    align-items: center;
  }

  :global(input[type="checkbox"].toggle) {
    appearance: none;
    width: 34px;
    height: 18px;
    background: var(--surface-3);
    border: 1px solid var(--border-2);
    border-radius: 20px;
    cursor: pointer;
    position: relative;
    transition: all 0.2s;
  }
  :global(input[type="checkbox"].toggle::after) {
    content: '';
    position: absolute;
    left: 2px;
    top: 2px;
    width: 12px;
    height: 12px;
    background: var(--text-muted);
    border-radius: 50%;
    transition: all 0.2s;
  }
  :global(input[type="checkbox"].toggle:checked) {
    background: var(--accent-dim);
    border-color: var(--accent);
  }
  :global(input[type="checkbox"].toggle:checked::after) {
    left: 18px;
    background: var(--accent);
  }

  .app {
    display: flex;
    min-height: 100vh;
  }

  main {
    flex: 1;
    padding: 2rem 2.5rem;
    min-width: 0;
    background: var(--bg);
  }
</style>
