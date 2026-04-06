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

  const SOURCE_TYPES = ['rss', 'newznab', 'torznab']
  const empty = () => ({ name: '', source_type: 'rss', url: '', api_key: '', poll_interval_mins: 720, enabled: true })
  let form = empty()

  onMount(load)

  async function load() {
    try { sources = await api.sources.list() }
    catch(e) { error = e.message }
  }

  function openNew()  { editing = null; form = empty(); showModal = true }
  function openEdit(s){ editing = s; form = { ...s, api_key: s.api_key ?? '' }; showModal = true }

  async function save() {
    try {
      editing ? await api.sources.update(editing.id, form)
              : await api.sources.create(form)
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
            <tr>
              <td class="name-cell">{s.name}</td>
              <td><span class="badge badge-{s.source_type}">{s.source_type}</span></td>
              <td class="url-cell mono">{s.url}</td>
              <td class="muted">{s.poll_interval_mins}m</td>
              <td class="muted mono">{fmt(s.last_polled_at)}</td>
              <td>
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
  <Modal title="{editing ? 'Edit' : 'New'} Source" onClose={() => showModal = false}>
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
</style>
