<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'
  import Modal from '../lib/Modal.svelte'
  import FormField from '../lib/FormField.svelte'

  let sources = []
  let error = null
  let showModal = false
  let editing = null
  let testResult = {}

  const SOURCE_TYPES = ['rss', 'newznab', 'torznab', 'prowlarr']
  const empty = () => ({ name: '', source_type: 'rss', url: '', api_key: '', poll_interval_mins: 720, enabled: true, categories: '' })
  let form = empty()
  let availableCats = []
  let loadingCats = false
  let catsError = null
  let catSearch = ''
  let showAdvancedCats = false

  // Newznab top-level category buckets (1000s, 2000s, …) → display label
  const CAT_GROUPS = [
    { base: 1000, label: 'Console', color: '#a78bfa' },
    { base: 2000, label: 'Movies',  color: '#60a5fa' },
    { base: 3000, label: 'Audio',   color: '#34d399' },
    { base: 4000, label: 'PC',      color: '#f97316' },
    { base: 5000, label: 'TV',      color: '#f472b6' },
    { base: 6000, label: 'XXX',     color: '#ef4444' },
    { base: 7000, label: 'Books',   color: '#fbbf24' },
    { base: 8000, label: 'Other',   color: '#94a3b8' },
  ]
  function groupOf(id) {
    const base = Math.floor(id / 1000) * 1000
    return CAT_GROUPS.find(g => g.base === base) || { base, label: `Cat ${base}`, color: '#64748b' }
  }

  onMount(load)

  async function load() {
    try { sources = await api.sources.list() }
    catch(e) { error = e.message }
  }

  function openNew()  {
    editing = null; form = empty()
    availableCats = []; catsError = null; catSearch = ''; showAdvancedCats = false
    showModal = true
  }
  function openEdit(s){
    editing = s; form = { ...s, api_key: s.api_key ?? '', categories: s.categories ?? '' }
    availableCats = []; catsError = null; catSearch = ''; showAdvancedCats = false
    showModal = true
    // Auto-load categories for saved Prowlarr sources — no hidden button.
    if (s.source_type === 'prowlarr') loadAvailableCats()
  }

  async function loadAvailableCats() {
    if (!editing || editing.source_type !== 'prowlarr') {
      catsError = 'Save the source first, then re-open to pick categories.'
      return
    }
    loadingCats = true; catsError = null
    try {
      const r = await api.sources.categories(editing.id)
      availableCats = (r.categories || []).sort((a, b) => a.id - b.id)
    } catch(e) { catsError = e.message }
    finally { loadingCats = false }
  }

  // Current selection as a Set of numeric IDs (for fast membership checks).
  $: selectedIds = new Set(
    (form.categories || '')
      .split(',')
      .map(s => parseInt(s.trim(), 10))
      .filter(n => !Number.isNaN(n))
  )

  function writeSelection(set) {
    form.categories = [...set].sort((a, b) => a - b).join(',')
  }

  function toggleCat(id) {
    const next = new Set(selectedIds)
    if (next.has(id)) next.delete(id); else next.add(id)
    writeSelection(next)
  }
  function selectGroup(baseCat, select) {
    const next = new Set(selectedIds)
    for (const c of availableCats) {
      if (Math.floor(c.id / 1000) * 1000 === baseCat) {
        if (select) next.add(c.id); else next.delete(c.id)
      }
    }
    writeSelection(next)
  }
  function clearAllCats() { writeSelection(new Set()) }

  // Availability grouped by newznab base (1000/2000/…), filtered by search.
  $: groupedCats = (() => {
    const q = catSearch.trim().toLowerCase()
    const filtered = availableCats.filter(c =>
      !q || c.name.toLowerCase().includes(q) || String(c.id).includes(q)
    )
    const buckets = new Map()
    for (const c of filtered) {
      const g = groupOf(c.id)
      if (!buckets.has(g.base)) buckets.set(g.base, { ...g, items: [] })
      buckets.get(g.base).items.push(c)
    }
    return [...buckets.values()].sort((a, b) => a.base - b.base)
  })()

  $: selectedCount = selectedIds.size

  async function save() {
    try {
      if (editing) {
        const payload = {
          name: form.name,
          source_type: form.source_type,
          url: form.url,
          api_key: form.api_key,
          poll_interval_mins: form.poll_interval_mins,
          enabled: form.enabled,
          categories: form.categories || null,
        }
        await api.sources.update(editing.id, payload)
      } else {
        await api.sources.create(form)
      }
      showModal = false
      await load()
    } catch(e) { error = e.message }
  }

  async function remove(id) {
    if (!confirm('Delete this source?')) return
    try { await api.sources.delete(id); await load() }
    catch(e) { error = e.message }
  }

  async function testNow(id) {
    testResult = { ...testResult, [id]: { status: 'pending', msg: 'Testing...' } }
    try {
      const r = await api.sources.test(id)
      testResult = { ...testResult, [id]: { status: 'ok', msg: `${r.item_count} items returned` } }
    } catch(e) {
      testResult = { ...testResult, [id]: { status: 'err', msg: e.message } }
    }
  }

  const fmt = dt => dt ? new Date(dt).toLocaleString() : 'Never'

  function relativeTime(iso) {
    if (!iso) return 'never'
    const diffMs = Date.now() - new Date(iso).getTime()
    if (diffMs < 0) return 'just now'
    const s = Math.floor(diffMs / 1000)
    if (s < 60)     return `${s}s ago`
    const m = Math.floor(s / 60)
    if (m < 60)     return `${m}m ago`
    const h = Math.floor(m / 60)
    if (h < 24)     return `${h}h ago`
    const d = Math.floor(h / 24)
    return `${d}d ago`
  }
</script>

<div class="page">
  <div class="page-header">
    <h1 class="page-title">Sources</h1>
    <button class="btn btn-primary" on:click={openNew}>+ New Source</button>
  </div>

  {#if error}<p class="error-msg">{error}</p>{/if}

  <div class="table-wrap">
    {#if sources.length === 0}
      <div class="empty-state">No sources configured. Add one to begin polling.</div>
    {:else}
      <table class="data-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Type</th>
            <th>URL</th>
            <th>Interval</th>
            <th>Last Polled</th>
            <th>Enabled</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each sources as s}
            <tr class:has-error={!!s.last_error}>
              <td class="name-cell" data-label="Name">{s.name}</td>
              <td data-label="Type"><span class="badge badge-{s.source_type}">{s.source_type}</span></td>
              <td class="url-cell mono" data-label="URL">{s.url}</td>
              <td class="muted" data-label="Interval">{s.poll_interval_mins}m</td>
              <td class="muted mono" data-label="Last Polled">{fmt(s.last_polled_at)}</td>
              <td data-label="Enabled">
                <span class="status-dot {s.enabled ? 'on' : 'off'}"></span>
              </td>
              <td>
                <div class="actions-cell">
                  <button class="btn btn-ghost" on:click={() => testNow(s.id)}>Test</button>
                  <button class="btn btn-ghost" on:click={() => openEdit(s)}>Edit</button>
                  <button class="btn btn-danger" on:click={() => remove(s.id)}>Delete</button>
                </div>
              </td>
            </tr>
            {#if s.last_error}
              <tr class="health-row health-error">
                <td colspan="7">
                  <span class="health-icon" aria-hidden="true">⚠</span>
                  <span class="health-label">Last error</span>
                  <span class="health-msg">{s.last_error}</span>
                  {#if s.last_success_at}
                    <span class="health-sep">·</span>
                    <span class="health-aux">last success {relativeTime(s.last_success_at)}</span>
                  {/if}
                </td>
              </tr>
            {:else if s.last_success_at}
              <tr class="health-row health-ok">
                <td colspan="7">
                  <span class="health-aux">last success {relativeTime(s.last_success_at)}</span>
                </td>
              </tr>
            {/if}
            {#if testResult[s.id]}
              <tr class="test-row">
                <td colspan="7">
                  <span class="test-result {testResult[s.id].status}">
                    {testResult[s.id].msg}
                  </span>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

{#if showModal}
  <Modal title="{editing ? 'Edit' : 'New'} Source" onClose={() => showModal = false} wide={form.source_type === 'prowlarr'}>
    <FormField label="Name">
      <input bind:value={form.name} placeholder="e.g. My Prowlarr" />
    </FormField>
    <FormField label="Type">
      <select bind:value={form.source_type}>
        {#each SOURCE_TYPES as t}<option value={t}>{t}</option>{/each}
      </select>
    </FormField>
    <FormField label="URL">
      <input bind:value={form.url} placeholder="https://..." />
    </FormField>
    <FormField label="API Key" hint="Optional">
      <input bind:value={form.api_key} placeholder="(optional)" />
    </FormField>
    <FormField label="Poll Interval (minutes)">
      <input type="number" bind:value={form.poll_interval_mins} min="1" />
    </FormField>
    <FormField label="Enabled">
      <input type="checkbox" class="toggle" bind:checked={form.enabled} />
    </FormField>

    {#if form.source_type === 'prowlarr' || form.source_type === 'torznab' || form.source_type === 'newznab'}
      <section class="cat-section">
        <div class="cat-section-head">
          <div class="cat-section-title">
            <span class="cat-section-label">Category Filter</span>
            <span class="cat-section-count" class:empty={selectedCount === 0}>
              {#if selectedCount === 0}
                all categories
              {:else}
                {selectedCount} selected
              {/if}
            </span>
          </div>
          <div class="cat-section-actions">
            {#if form.source_type === 'prowlarr' && editing}
              <button type="button" class="cat-btn" on:click={loadAvailableCats} disabled={loadingCats}>
                {loadingCats ? 'Loading…' : (availableCats.length ? '↻ Reload' : '↓ Load from Prowlarr')}
              </button>
            {/if}
            {#if selectedCount > 0}
              <button type="button" class="cat-btn cat-btn-clear" on:click={clearAllCats}>Clear</button>
            {/if}
          </div>
        </div>

        <p class="cat-section-hint">
          Narrow what this source returns. Empty = everything. {form.source_type === 'prowlarr' ? 'Categories are loaded from Prowlarr automatically when you open a saved source.' : 'Enter comma-separated newznab category IDs below.'}
        </p>

        {#if catsError}
          <div class="cat-alert">{catsError}</div>
        {/if}

        {#if form.source_type === 'prowlarr' && editing}
          {#if availableCats.length === 0 && !loadingCats && !catsError}
            <div class="cat-empty">No categories loaded yet. Click "Load from Prowlarr" above.</div>
          {:else if availableCats.length}
            <div class="cat-search-bar">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
              <input
                type="text"
                class="cat-search-input"
                bind:value={catSearch}
                placeholder="Search {availableCats.length} categories…"
              />
              {#if catSearch}
                <button type="button" class="cat-search-clear" on:click={() => catSearch = ''} aria-label="Clear search">✕</button>
              {/if}
            </div>

            <div class="cat-groups">
              {#each groupedCats as bucket (bucket.base)}
                {@const groupSelected = bucket.items.filter(i => selectedIds.has(i.id)).length}
                <div class="cat-bucket" style="--bucket-color: {bucket.color}">
                  <div class="cat-bucket-head">
                    <div class="cat-bucket-label">
                      <span class="cat-bucket-dot"></span>
                      <span class="cat-bucket-name">{bucket.label}</span>
                      <span class="cat-bucket-base mono">{bucket.base}</span>
                    </div>
                    <div class="cat-bucket-actions">
                      <span class="cat-bucket-count">
                        {groupSelected}/{bucket.items.length}
                      </span>
                      {#if groupSelected < bucket.items.length}
                        <button type="button" class="cat-mini-btn" on:click={() => selectGroup(bucket.base, true)}>All</button>
                      {/if}
                      {#if groupSelected > 0}
                        <button type="button" class="cat-mini-btn" on:click={() => selectGroup(bucket.base, false)}>None</button>
                      {/if}
                    </div>
                  </div>
                  <div class="cat-bucket-items">
                    {#each bucket.items as c (c.id)}
                      <button
                        type="button"
                        class="cat-tile"
                        class:on={selectedIds.has(c.id)}
                        on:click={() => toggleCat(c.id)}
                      >
                        <span class="cat-tile-check">
                          {#if selectedIds.has(c.id)}
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
                          {/if}
                        </span>
                        <span class="cat-tile-id">{c.id}</span>
                        <span class="cat-tile-name">{c.name}</span>
                      </button>
                    {/each}
                  </div>
                </div>
              {/each}
              {#if groupedCats.length === 0}
                <div class="cat-empty">No categories match "{catSearch}"</div>
              {/if}
            </div>
          {/if}
        {/if}

        <details class="cat-advanced" bind:open={showAdvancedCats}>
          <summary>Advanced: edit raw CSV</summary>
          <input class="cat-raw-input" bind:value={form.categories} placeholder="e.g. 1000,4050" />
          <p class="cat-advanced-hint">Comma-separated newznab category IDs. Changes here sync with the picker above.</p>
        </details>
      </section>
    {/if}

    <svelte:fragment slot="footer">
      <button class="btn" on:click={() => showModal = false}>Cancel</button>
      <button class="btn btn-primary" on:click={save}>Save</button>
    </svelte:fragment>
  </Modal>
{/if}

<style>
  .name-cell { font-weight: 600; }
  .muted     { color: var(--text-muted); font-size: 0.85rem; }
  .mono      { font-family: var(--font-mono); font-size: 0.8rem; }

  .url-cell {
    max-width: 220px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .test-row td {
    background: var(--surface-2);
    padding: 0.5rem 0.875rem;
  }

  .test-result {
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }
  .test-result.ok      { color: var(--green); }
  .test-result.err     { color: var(--red); }
  .test-result.pending { color: var(--text-muted); }

  /* Source health rows */
  tr.has-error td {
    border-bottom-color: transparent;
  }
  .health-row td {
    padding: 0.5rem 0.875rem;
    font-size: 0.78rem;
    font-family: var(--font-mono);
    border-bottom: 1px solid var(--border);
  }
  .health-row.health-error td {
    background: var(--red-dim);
    color: var(--red);
    border-left: 3px solid var(--red);
  }
  .health-row.health-ok td {
    background: var(--surface);
    color: var(--text-dim);
  }
  .health-icon {
    display: inline-block;
    margin-right: 0.4rem;
    font-size: 0.9rem;
  }
  .health-label {
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-right: 0.5rem;
  }
  .health-msg {
    color: var(--red);
    opacity: 0.95;
    word-break: break-word;
  }
  .health-sep {
    margin: 0 0.5rem;
    opacity: 0.5;
  }
  .health-aux {
    color: var(--text-muted);
  }

  /* ───────── Category picker ───────── */
  .cat-section {
    margin-top: 0.25rem;
    padding: 1rem 1.1rem 1.1rem;
    background: var(--surface-2);
    border: 1px solid var(--border-2);
    border-radius: 10px;
    display: flex; flex-direction: column; gap: 0.75rem;
  }
  .cat-section-head {
    display: flex; align-items: center; justify-content: space-between; gap: 0.75rem;
    flex-wrap: wrap;
  }
  .cat-section-title { display: flex; align-items: baseline; gap: 0.7rem; }
  .cat-section-label {
    font-family: var(--font-head);
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--text);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .cat-section-count {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    padding: 0.2rem 0.55rem;
    background: var(--accent-dim);
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .cat-section-count.empty {
    background: transparent;
    color: var(--text-muted);
    border-color: var(--border);
  }
  .cat-section-actions { display: flex; gap: 0.4rem; }
  .cat-btn {
    padding: 0.42rem 0.8rem;
    background: transparent;
    border: 1px solid var(--border-2);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.12s ease;
  }
  .cat-btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); background: var(--accent-dim); }
  .cat-btn:disabled { opacity: 0.4; cursor: wait; }
  .cat-btn-clear:hover:not(:disabled) { border-color: var(--red); color: var(--red); background: var(--red-dim); }

  .cat-section-hint {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .cat-alert {
    padding: 0.6rem 0.8rem;
    background: var(--red-dim);
    color: var(--red);
    border: 1px solid var(--red);
    border-radius: 6px;
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }

  .cat-empty {
    padding: 1.5rem;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.82rem;
    font-style: italic;
    border: 1px dashed var(--border);
    border-radius: 6px;
  }

  /* Search bar */
  .cat-search-bar {
    display: flex; align-items: center; gap: 0.5rem;
    padding: 0.55rem 0.75rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    transition: border-color 0.12s;
  }
  .cat-search-bar:focus-within { border-color: var(--accent); }
  .cat-search-bar svg { color: var(--text-muted); flex-shrink: 0; }
  .cat-search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 0.82rem;
  }
  .cat-search-input::placeholder { color: var(--text-muted); }
  .cat-search-clear {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 0.3rem;
    font-size: 0.8rem;
  }
  .cat-search-clear:hover { color: var(--red); }

  /* Grouped buckets */
  .cat-groups {
    display: flex; flex-direction: column; gap: 0.75rem;
    max-height: 480px;
    overflow-y: auto;
    padding-right: 0.3rem;
  }
  .cat-bucket {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    border-left: 3px solid var(--bucket-color);
    overflow: hidden;
    flex-shrink: 0;
  }
  .cat-bucket-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.6rem 0.9rem;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
  }
  .cat-bucket-label { display: flex; align-items: center; gap: 0.55rem; }
  .cat-bucket-dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--bucket-color);
    box-shadow: 0 0 8px var(--bucket-color);
  }
  .cat-bucket-name {
    font-family: var(--font-head);
    font-size: 0.88rem;
    font-weight: 700;
    color: var(--text);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .cat-bucket-base {
    font-size: 0.7rem;
    color: var(--text-muted);
    opacity: 0.7;
  }
  .cat-bucket-actions { display: flex; align-items: center; gap: 0.5rem; }
  .cat-bucket-count {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-muted);
    font-weight: 600;
  }
  .cat-mini-btn {
    padding: 0.25rem 0.55rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 700;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.1s;
  }
  .cat-mini-btn:hover { border-color: var(--bucket-color); color: var(--bucket-color); }

  .cat-bucket-items {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(220px, 100%), 1fr));
    gap: 0.4rem;
    padding: 0.7rem 0.9rem;
  }
  .cat-tile {
    display: flex; align-items: center; gap: 0.5rem;
    padding: 0.55rem 0.7rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    text-align: left;
    cursor: pointer;
    transition: all 0.12s ease;
    min-height: 40px;
  }
  .cat-tile:hover { border-color: var(--bucket-color); background: var(--surface-3); }
  .cat-tile.on {
    border-color: var(--bucket-color);
    background: color-mix(in srgb, var(--bucket-color) 12%, var(--surface));
    box-shadow: inset 0 0 0 1px var(--bucket-color);
  }
  .cat-tile-check {
    width: 18px; height: 18px;
    display: flex; align-items: center; justify-content: center;
    border: 1.5px solid var(--border-2);
    border-radius: 4px;
    flex-shrink: 0;
    color: var(--bucket-color);
    transition: all 0.12s;
  }
  .cat-tile.on .cat-tile-check {
    border-color: var(--bucket-color);
    background: var(--bucket-color);
    color: var(--bg, #0a0a0a);
  }
  .cat-tile-id {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-muted);
    font-weight: 700;
    flex-shrink: 0;
  }
  .cat-tile.on .cat-tile-id { color: var(--bucket-color); }
  .cat-tile-name {
    font-size: 0.8rem;
    color: var(--text);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Advanced CSV fallback */
  .cat-advanced {
    margin-top: 0.25rem;
    padding-top: 0.75rem;
    border-top: 1px dashed var(--border);
  }
  .cat-advanced summary {
    cursor: pointer;
    font-size: 0.76rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.3rem 0;
    user-select: none;
  }
  .cat-advanced summary:hover { color: var(--accent); }
  .cat-raw-input {
    width: 100%;
    margin-top: 0.5rem;
    padding: 0.55rem 0.75rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 0.82rem;
  }
  .cat-raw-input:focus { outline: none; border-color: var(--accent); }
  .cat-advanced-hint {
    margin: 0.5rem 0 0;
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  /* ───────── Responsive ───────── */
  @media (max-width: 720px) {
    .url-cell { max-width: none; white-space: normal; overflow-wrap: anywhere; }

    .cat-section { padding: 0.85rem 0.85rem 0.95rem; }
    .cat-section-head { gap: 0.5rem; }
    .cat-section-actions { width: 100%; justify-content: flex-start; }
    .cat-groups { max-height: 55vh; }
    .cat-bucket-head {
      flex-wrap: wrap;
      gap: 0.4rem 0.6rem;
      padding: 0.55rem 0.7rem;
    }
    .cat-bucket-items { padding: 0.55rem 0.7rem; }
    .cat-tile { min-height: 44px; }
  }
</style>
