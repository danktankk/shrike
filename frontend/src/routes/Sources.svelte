<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let sources = []
  let error = null
  let showModal = false
  let editing = null
  let testResult = {}

  const SOURCE_TYPES = ['rss', 'newznab', 'torznab']
  const empty = () => ({ name: '', source_type: 'rss', url: '', api_key: '', poll_interval_mins: 720, enabled: true })
  let form = empty()

  onMount(load)

  async function load() {
    try { sources = await api.sources.list() }
    catch(e) { error = e.message }
  }

  function openNew() { editing = null; form = empty(); showModal = true }
  function openEdit(s) { editing = s; form = { ...s, api_key: s.api_key ?? '' }; showModal = true }

  async function save() {
    try {
      if (editing) await api.sources.update(editing.id, form)
      else await api.sources.create(form)
      showModal = false; await load()
    } catch(e) { error = e.message }
  }

  async function remove(id) {
    if (!confirm('Delete this source?')) return
    try { await api.sources.delete(id); await load() }
    catch(e) { error = e.message }
  }

  async function testNow(id) {
    testResult = { ...testResult, [id]: 'Testing...' }
    try {
      const r = await api.sources.test(id)
      testResult = { ...testResult, [id]: `${r.item_count} items returned` }
    } catch(e) {
      testResult = { ...testResult, [id]: `Error: ${e.message}` }
    }
  }
</script>

<div>
  <div class="header"><h2>Sources</h2><button on:click={openNew}>+ Add</button></div>
  {#if error}<p class="error">{error}</p>{/if}

  <table>
    <thead><tr><th>Name</th><th>Type</th><th>URL</th><th>Interval</th><th>Last Polled</th><th>Enabled</th><th></th></tr></thead>
    <tbody>
      {#each sources as s}
        <tr>
          <td>{s.name}</td>
          <td><span class="badge {s.source_type}">{s.source_type}</span></td>
          <td class="url">{s.url}</td>
          <td>{s.poll_interval_mins}m</td>
          <td>{s.last_polled_at ? new Date(s.last_polled_at).toLocaleString() : 'Never'}</td>
          <td>{s.enabled ? '✓' : '—'}</td>
          <td class="actions">
            <button on:click={() => testNow(s.id)}>Test</button>
            <button on:click={() => openEdit(s)}>Edit</button>
            <button class="danger" on:click={() => remove(s.id)}>Delete</button>
          </td>
        </tr>
        {#if testResult[s.id]}
          <tr class="test-row"><td colspan="7">{testResult[s.id]}</td></tr>
        {/if}
      {/each}
    </tbody>
  </table>

  {#if showModal}
    <div class="overlay" on:click|self={() => showModal = false}>
      <div class="modal">
        <h3>{editing ? 'Edit' : 'New'} Source</h3>
        <label>Name <input bind:value={form.name} /></label>
        <label>Type
          <select bind:value={form.source_type}>
            {#each SOURCE_TYPES as t}<option value={t}>{t}</option>{/each}
          </select>
        </label>
        <label>URL <input bind:value={form.url} placeholder="https://..." /></label>
        <label>API Key <input bind:value={form.api_key} placeholder="(optional)" /></label>
        <label>Poll Interval (minutes) <input type="number" bind:value={form.poll_interval_mins} /></label>
        <label>Enabled <input type="checkbox" bind:checked={form.enabled} /></label>
        <div class="modal-actions">
          <button on:click={() => showModal = false}>Cancel</button>
          <button class="primary" on:click={save}>Save</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .header { display: flex; align-items: center; justify-content: space-between; }
  h2 { margin-top: 0; }
  table { width: 100%; border-collapse: collapse; }
  th, td { border: 1px solid #ddd; padding: 0.4rem 0.6rem; font-size: 0.85rem; }
  th { background: #f5f5f5; }
  .url { max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge { padding: 2px 6px; border-radius: 3px; font-size: 0.75rem; font-weight: bold; }
  .badge.rss { background: #e8f5e9; color: #2e7d32; }
  .badge.newznab { background: #e3f2fd; color: #1565c0; }
  .badge.torznab { background: #fce4ec; color: #880e4f; }
  .test-row td { background: #f9f9f9; color: #555; font-size: 0.82rem; padding-left: 1rem; }
  .actions { white-space: nowrap; }
  .danger { color: #c00; }
  .primary { background: #5566dd; color: #fff; border: none; padding: 0.4rem 1rem; border-radius: 4px; }
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
  .modal { background: #fff; padding: 1.5rem; border-radius: 8px; min-width: 420px; display: flex; flex-direction: column; gap: 0.8rem; }
  .modal label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; }
  .modal input, .modal select { padding: 0.3rem; border: 1px solid #ccc; border-radius: 4px; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
  .error { color: red; }
</style>
