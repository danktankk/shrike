<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let matches = []
  let terms = []
  let sources = []
  let filterTerm = ''
  let filterSource = ''
  let error = null

  onMount(async () => {
    try {
      ;[matches, terms, sources] = await Promise.all([
        api.matches.list({ limit: 100 }),
        api.searchTerms.list(),
        api.sources.list(),
      ])
    } catch (e) {
      error = e.message
    }
  })

  $: filtered = matches.filter(m => {
    if (filterTerm   && m.search_term_id != filterTerm)   return false
    if (filterSource && m.source_id      != filterSource) return false
    return true
  })

  const termName   = id => terms.find(t => t.id === id)?.name   ?? id
  const sourceName = id => sources.find(s => s.id === id)?.name ?? id
  const channels   = json => { try { return JSON.parse(json || '[]').join(', ') || '—' } catch { return '—' } }
  const fmt        = dt  => dt ? new Date(dt).toLocaleString() : '—'
</script>

<div class="page">
  <div class="page-header">
    <h1 class="page-title">Dashboard</h1>
    <div class="header-meta">
      {filtered.length} match{filtered.length !== 1 ? 'es' : ''}
    </div>
  </div>

  {#if error}
    <p class="error-msg">{error}</p>
  {/if}

  <div class="filters">
    <select bind:value={filterTerm} class="filter-select">
      <option value="">All terms</option>
      {#each terms as t}<option value={t.id}>{t.name}</option>{/each}
    </select>
    <select bind:value={filterSource} class="filter-select">
      <option value="">All sources</option>
      {#each sources as s}<option value={s.id}>{s.name}</option>{/each}
    </select>
    {#if filterTerm || filterSource}
      <button class="btn btn-ghost" on:click={() => { filterTerm = ''; filterSource = '' }}>
        Clear filters
      </button>
    {/if}
  </div>

  <div class="table-wrap">
    {#if filtered.length === 0}
      <div class="empty-state">
        {matches.length === 0 ? 'No matches yet — sources will poll on their schedule.' : 'No matches for selected filters.'}
      </div>
    {:else}
      <table class="data-table">
        <thead>
          <tr>
            <th>Matched</th>
            <th>Term</th>
            <th>Title</th>
            <th>Source</th>
            <th>Channels</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as m}
            <tr>
              <td class="mono muted">{fmt(m.matched_at)}</td>
              <td><span class="term-pill">{termName(m.search_term_id)}</span></td>
              <td class="title-cell">
                {#if m.item_url}
                  <a href={m.item_url} target="_blank" rel="noopener">{m.item_title}</a>
                {:else}
                  {m.item_title}
                {/if}
              </td>
              <td class="muted">{sourceName(m.source_id)}</td>
              <td class="mono muted">{channels(m.notification_channels)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .header-meta {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    padding: 0.3rem 0.75rem;
    border-radius: 20px;
  }

  .filters {
    display: flex;
    gap: 0.6rem;
    align-items: center;
  }

  .filter-select {
    padding: 0.45rem 2rem 0.45rem 0.75rem;
    background: var(--surface);
    border: 1px solid var(--border-2);
    border-radius: 7px;
    color: var(--text);
    font-family: var(--font-body);
    font-size: 0.85rem;
    cursor: pointer;
    outline: none;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'%3E%3Cpath fill='%235a5a80' d='M0 0l5 6 5-6z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.6rem center;
    transition: border-color 0.15s;
  }
  .filter-select:focus { border-color: var(--accent); }

  .term-pill {
    background: var(--accent-dim);
    color: var(--accent);
    padding: 2px 8px;
    border-radius: 20px;
    font-size: 0.78rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .title-cell {
    max-width: 320px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mono  { font-family: var(--font-mono); font-size: 0.8rem; }
  .muted { color: var(--text-muted); }
</style>
