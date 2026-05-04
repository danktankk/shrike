<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let entries = []
  let error = null
  let newPattern = ''
  let busy = false

  onMount(load)

  async function load() {
    try { entries = await api.blocklist.list() }
    catch (e) { error = e.message }
  }

  async function add() {
    const p = newPattern.trim()
    if (!p) return
    busy = true
    error = null
    try {
      await api.blocklist.create(p)
      newPattern = ''
      await load()
    } catch (e) {
      error = e.message
    } finally {
      busy = false
    }
  }

  async function remove(id, pattern) {
    if (!confirm(`Remove "${pattern}" from the blocklist?`)) return
    try {
      await api.blocklist.delete(id)
      await load()
    } catch (e) { error = e.message }
  }

  function onKeydown(e) {
    if (e.key === 'Enter') add()
  }
</script>

<div class="page">
  <div class="page-header">
    <h1 class="page-title">SGDB Blocklist</h1>
  </div>

  <p class="lede">
    Patterns suppress matching SteamGridDB and Steam storefront hits during
    art/metadata enrichment. Whole-word, case-insensitive. Sequel markers
    (digits ≥ 2, roman numerals II+, <code>v2</code>+) bypass the block —
    so <code>hypervisor</code> stops "Hypervisor" but lets "Hypervisor 2"
    or "Hypervisor: III" through.
  </p>

  {#if error}<p class="error-msg">{error}</p>{/if}

  <div class="add-row">
    <input
      bind:value={newPattern}
      on:keydown={onKeydown}
      placeholder="e.g. hypervisor"
      disabled={busy}
    />
    <button class="btn btn-primary" on:click={add} disabled={busy || !newPattern.trim()}>
      Add
    </button>
  </div>

  <div class="table-wrap">
    {#if entries.length === 0}
      <div class="empty-state">Blocklist is empty.</div>
    {:else}
      <table class="data-table">
        <thead>
          <tr>
            <th>Pattern</th>
            <th>Added</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each entries as e}
            <tr>
              <td data-label="Pattern"><code>{e.pattern}</code></td>
              <td class="muted" data-label="Added">{new Date(e.created_at).toLocaleDateString()}</td>
              <td>
                <button class="btn btn-danger" on:click={() => remove(e.id, e.pattern)}>
                  Remove
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .lede { color: var(--text-muted); max-width: 60ch; line-height: 1.5; margin: 0 0 1.25rem; }
  .lede code { font-family: var(--font-mono); font-size: 0.85em; color: var(--accent); }
  .add-row { display: flex; gap: 0.5rem; margin-bottom: 1.25rem; max-width: 36rem; }
  .add-row input { flex: 1; }
  .muted { color: var(--text-muted); font-size: 0.85rem; }
</style>
