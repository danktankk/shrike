<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let terms = []
  let error = null
  let showModal = false
  let editing = null

  const empty = () => ({ name: '', query: '', max_age_days: 30, disallowed_keywords: '', enabled: true })
  let form = empty()

  onMount(load)

  async function load() {
    try { terms = await api.searchTerms.list() }
    catch (e) { error = e.message }
  }

  function openNew() { editing = null; form = empty(); showModal = true }
  function openEdit(t) { editing = t; form = { ...t, disallowed_keywords: t.disallowed_keywords ?? '' }; showModal = true }

  async function save() {
    try {
      if (editing) await api.searchTerms.update(editing.id, form)
      else await api.searchTerms.create(form)
      showModal = false
      await load()
    } catch(e) { error = e.message }
  }

  async function remove(id) {
    if (!confirm('Delete this term?')) return
    try { await api.searchTerms.delete(id); await load() }
    catch(e) { error = e.message }
  }

  async function toggleEnabled(t) {
    try { await api.searchTerms.update(t.id, { ...t, enabled: !t.enabled }); await load() }
    catch(e) { error = e.message }
  }
</script>

<div>
  <div class="header">
    <h2>Search Terms</h2>
    <button on:click={openNew}>+ Add</button>
  </div>
  {#if error}<p class="error">{error}</p>{/if}

  <table>
    <thead><tr><th>Name</th><th>Query</th><th>Max Age</th><th>Blocked Keywords</th><th>Enabled</th><th></th></tr></thead>
    <tbody>
      {#each terms as t}
        <tr>
          <td>{t.name}</td>
          <td><code>{t.query}</code></td>
          <td>{t.max_age_days ?? 30}d</td>
          <td>{t.disallowed_keywords ?? '—'}</td>
          <td><input type="checkbox" checked={t.enabled} on:change={() => toggleEnabled(t)} /></td>
          <td class="actions">
            <button on:click={() => openEdit(t)}>Edit</button>
            <button class="danger" on:click={() => remove(t.id)}>Delete</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if showModal}
    <div class="overlay" on:click|self={() => showModal = false}>
      <div class="modal">
        <h3>{editing ? 'Edit' : 'New'} Search Term</h3>
        <label>Name <input bind:value={form.name} /></label>
        <label>Query <input bind:value={form.query} placeholder="e.g. Elden Ring" /></label>
        <label>Max Age (days) <input type="number" bind:value={form.max_age_days} /></label>
        <label>Disallowed Keywords (comma-separated) <input bind:value={form.disallowed_keywords} placeholder="trainer,crack,repack" /></label>
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
  th, td { border: 1px solid #ddd; padding: 0.4rem 0.6rem; }
  th { background: #f5f5f5; }
  .actions { white-space: nowrap; }
  .danger { color: #c00; }
  .primary { background: #5566dd; color: #fff; border: none; padding: 0.4rem 1rem; border-radius: 4px; }
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; }
  .modal { background: #fff; padding: 1.5rem; border-radius: 8px; min-width: 400px; display: flex; flex-direction: column; gap: 0.8rem; }
  .modal label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; }
  .modal input { padding: 0.3rem; border: 1px solid #ccc; border-radius: 4px; }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
  .error { color: red; }
</style>
