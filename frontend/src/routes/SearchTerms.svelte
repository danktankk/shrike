<script>
  import { onMount, onDestroy } from 'svelte'
  import { api } from '../lib/api.js'
  import Modal from '../lib/Modal.svelte'
  import FormField from '../lib/FormField.svelte'

  let terms = []
  let error = null
  let showModal = false
  let editing = null

  // Per-row scan state: term_id → { state: 'idle'|'scanning'|'done'|'error', msg?: string }
  let scanStates = {}
  const scanTimers = new Map()
  let destroyed = false

  const empty = () => ({ name: '', query: '', max_age_days: 30, disallowed_keywords: '', enabled: true })
  let form = empty()

  onMount(load)
  onDestroy(() => {
    destroyed = true
    for (const id of scanTimers.values()) clearTimeout(id)
    scanTimers.clear()
  })

  async function load() {
    try { terms = await api.searchTerms.list() }
    catch (e) { error = e.message }
  }

  function openNew()  { editing = null; form = empty(); showModal = true }
  function openEdit(t){ editing = t; form = { ...t, disallowed_keywords: t.disallowed_keywords ?? '' }; showModal = true }

  async function save() {
    try {
      if (editing) {
        const payload = {
          name: form.name,
          query: form.query,
          enabled: form.enabled,
          max_age_days: form.max_age_days,
          disallowed_keywords: form.disallowed_keywords,
        }
        await api.searchTerms.update(editing.id, payload)
      } else {
        await api.searchTerms.create(form)
      }
      showModal = false
      await load()
    } catch(e) { error = e.message }
  }

  async function remove(id) {
    if (!confirm('Delete this search term?')) return
    try { await api.searchTerms.delete(id); await load() }
    catch(e) { error = e.message }
  }

  async function toggleEnabled(t) {
    try { await api.searchTerms.update(t.id, { ...t, enabled: !t.enabled }); await load() }
    catch(e) { error = e.message }
  }

  async function scanTerm(t) {
    const current = scanStates[t.id]?.state
    if (current === 'scanning') return
    clearTimeout(scanTimers.get(t.id))
    scanStates = { ...scanStates, [t.id]: { state: 'scanning' } }
    try {
      const res = await api.searchTerms.scan(t.id)
      if (destroyed) return
      const found = res?.matches_found ?? 0
      scanStates = { ...scanStates, [t.id]: { state: 'done', msg: `${found} found` } }
    } catch (e) {
      if (destroyed) return
      scanStates = { ...scanStates, [t.id]: { state: 'error', msg: e.message } }
    }
    if (destroyed) return
    scanTimers.set(t.id, setTimeout(() => {
      const rest = { ...scanStates }
      delete rest[t.id]
      scanStates = rest
      scanTimers.delete(t.id)
    }, 4000))
  }
</script>

<div class="page">
  <div class="page-header">
    <h1 class="page-title">Search Terms</h1>
    <button class="btn btn-primary" on:click={openNew}>+ New Term</button>
  </div>

  {#if error}<p class="error-msg">{error}</p>{/if}

  <div class="table-wrap">
    {#if terms.length === 0}
      <div class="empty-state">No search terms yet. Add one to start watching.</div>
    {:else}
      <table class="data-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Query</th>
            <th>Max Age</th>
            <th>Blocked Keywords</th>
            <th>Enabled</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each terms as t}
            {@const st = scanStates[t.id]?.state ?? 'idle'}
            <tr>
              <td class="name-cell" data-label="Name">{t.name}</td>
              <td data-label="Query"><code>{t.query}</code></td>
              <td class="muted" data-label="Max Age">{t.max_age_days ?? 30}d</td>
              <td class="muted" data-label="Blocked">{t.disallowed_keywords || '—'}</td>
              <td data-label="Enabled">
                <input
                  type="checkbox"
                  class="toggle"
                  checked={t.enabled}
                  on:change={() => toggleEnabled(t)}
                />
              </td>
              <td>
                <div class="actions-cell">
                  <button
                    class="btn btn-ghost btn-scan-term {st}"
                    on:click={() => scanTerm(t)}
                    disabled={st === 'scanning'}
                    title={scanStates[t.id]?.msg ?? 'Scan all sources for this term only'}
                  >
                    {#if st === 'scanning'}Scanning…
                    {:else if st === 'done'}{scanStates[t.id].msg}
                    {:else if st === 'error'}Failed
                    {:else}Scan
                    {/if}
                  </button>
                  <button class="btn btn-ghost" on:click={() => openEdit(t)}>Edit</button>
                  <button class="btn btn-danger" on:click={() => remove(t.id)}>Delete</button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

{#if showModal}
  <Modal title="{editing ? 'Edit' : 'New'} Search Term" onClose={() => showModal = false}>
    <FormField label="Name">
      <input bind:value={form.name} placeholder="e.g. Elden Ring" />
    </FormField>
    <FormField label="Query" hint="Whole-word match, case-insensitive">
      <input bind:value={form.query} placeholder="e.g. elden ring" />
    </FormField>
    <FormField label="Max Age (days)">
      <input type="number" bind:value={form.max_age_days} min="1" />
    </FormField>
    <FormField label="Blocked Keywords" hint="Comma-separated, items containing these are skipped">
      <input bind:value={form.disallowed_keywords} placeholder="trainer,crack,repack" />
    </FormField>
    <FormField label="Enabled">
      <input type="checkbox" class="toggle" bind:checked={form.enabled} />
    </FormField>

    <svelte:fragment slot="footer">
      <button class="btn" on:click={() => showModal = false}>Cancel</button>
      <button class="btn btn-primary" on:click={save}>Save</button>
    </svelte:fragment>
  </Modal>
{/if}

<style>
  .name-cell { font-weight: 600; }
  .muted     { color: var(--text-muted); font-size: 0.85rem; }

  .btn-scan-term.scanning { border-color: var(--accent); color: var(--accent); background: var(--accent-dim); cursor: wait; }
  .btn-scan-term.done     { border-color: var(--green);  color: var(--green);  background: var(--green-dim); }
  .btn-scan-term.error    { border-color: var(--red);    color: var(--red);    background: var(--red-dim); }
</style>
