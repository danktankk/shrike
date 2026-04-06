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
    if (filterTerm && m.search_term_id != filterTerm) return false
    if (filterSource && m.source_id != filterSource) return false
    return true
  })

  function termName(id) { return terms.find(t => t.id === id)?.name ?? id }
  function sourceName(id) { return sources.find(s => s.id === id)?.name ?? id }
  function channels(json) {
    try { return JSON.parse(json || '[]').join(', ') || '—' } catch { return '—' }
  }
  function fmt(dt) { return dt ? new Date(dt).toLocaleString() : '—' }
</script>

<div>
  <h2>Dashboard</h2>
  {#if error}<p class="error">{error}</p>{/if}

  <div class="filters">
    <select bind:value={filterTerm}>
      <option value="">All terms</option>
      {#each terms as t}<option value={t.id}>{t.name}</option>{/each}
    </select>
    <select bind:value={filterSource}>
      <option value="">All sources</option>
      {#each sources as s}<option value={s.id}>{s.name}</option>{/each}
    </select>
  </div>

  {#if filtered.length === 0}
    <p class="empty">No matches yet.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Matched</th><th>Term</th><th>Title</th><th>Source</th><th>Channels</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as m}
          <tr>
            <td>{fmt(m.matched_at)}</td>
            <td>{termName(m.search_term_id)}</td>
            <td>{#if m.item_url}<a href={m.item_url} target="_blank">{m.item_title}</a>{:else}{m.item_title}{/if}</td>
            <td>{sourceName(m.source_id)}</td>
            <td>{channels(m.notification_channels)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  h2 { margin-top: 0; }
  .filters { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  select { padding: 0.3rem; }
  table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
  th, td { border: 1px solid #ddd; padding: 0.4rem 0.6rem; text-align: left; }
  th { background: #f5f5f5; }
  tr:hover td { background: #fafafa; }
  .empty { color: #888; }
  .error { color: red; }
  a { color: #5566dd; }
</style>
