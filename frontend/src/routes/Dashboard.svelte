<script>
  import { onMount, onDestroy } from 'svelte'
  import { slide } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { api } from '../lib/api.js'

  // ─── state ─────────────────────────────────────────────────────────────
  let matches  = []
  let terms    = []
  let sources  = []
  let filterTerm   = ''
  let filterSource = ''
  let error     = null
  let scanState = 'idle'  // 'idle' | 'scanning' | 'done' | 'error'
  let scanResult = null
  let scanTimer  = null

  // title(lowercased) -> GameRef | 'loading' | null
  //   GameRef (locked contract): { id, name, grid_url, hero_url, logo_url }
  //   Store URLs / release date / platforms live on /api/match/:id, not here.
  let artCache = new Map()
  let expanded = new Set()   // group keys currently open

  // ─── load ──────────────────────────────────────────────────────────────
  async function loadAll() {
    ;[matches, terms, sources] = await Promise.all([
      api.matches.list({ limit: 200 }),
      api.searchTerms.list(),
      api.sources.list(),
    ])
  }

  onMount(async () => {
    try { await loadAll() } catch (e) { error = e.message }
  })

  onDestroy(() => clearTimeout(scanTimer))

  async function scanNow() {
    if (scanState === 'scanning') return
    scanState = 'scanning'
    scanResult = null
    clearTimeout(scanTimer)
    try {
      scanResult = await api.scan()
      scanState = 'done'
      await loadAll()
    } catch (e) {
      scanResult = { error: e.message }
      scanState = 'error'
    }
    scanTimer = setTimeout(() => { scanState = 'idle'; scanResult = null }, 4000)
  }

  // ─── filter + group ────────────────────────────────────────────────────
  $: filtered = matches.filter(m => {
    if (filterTerm   && m.search_term_id !== Number(filterTerm))   return false
    if (filterSource && m.source_id      !== Number(filterSource)) return false
    return true
  })

  // groups keyed by normalizeTitle(item_title): { key, title, items[], latest, sourceIds:Set }
  $: groups = buildGroups(filtered)

  // Collapse release noise so "Borderlands.4.v1.0.2-CODEX" and
  // "Borderlands 4 v1.0.1 [FitGirl Repack]" share one bucket while still
  // preserving distinct SKUs like "Deluxe Edition" or "GOTY".
  function normalizeTitle(raw) {
    if (!raw) return ''
    let s = raw.toLowerCase()
    // Bracketed/parenthesized release-group tags
    s = s.replace(/[\[(][^\])]*(repack|codex|flt|rune|empress|dodi|fitgirl|tenoke|p2p|skidrow|plaza)[^\])]*[\])]/ig, '')
    // File extensions
    s = s.replace(/\.(rar|zip|7z|iso|exe|part\d+)\b/ig, '')
    // Collapse underscores to spaces early — `\b` doesn't fire at underscore
    // boundaries, so version/build/update strips would miss `_v1.0_` forms.
    // Dots stay for now; the version regex handles dot-separated numbers.
    s = s.replace(/_+/g, ' ')
    // Trailing release-group tag and everything after.
    s = s.replace(/[-.\s](codex|flt|rune|skidrow|plaza|hoodlum|razor1911|empress|tenoke|dodi|fitgirl|repack|p2p|anomaly|chronos|prophet|tinyiso)\b.*$/i, '')
    // Version markers: v1.2.3, v 1.2, version 1.2.3a
    s = s.replace(/\bv(ersion)?[\s.]*\d+([.-]\d+)*[a-z]?\b/ig, '')
    // Build markers: build 12345, build.12345, b12345
    s = s.replace(/\bbuild[\s.]*\d+\b/ig, '')
    s = s.replace(/\bb\d{4,}\b/ig, '')
    // Update / patch / hotfix markers (dot-separated too)
    s = s.replace(/\b(update|patch|hotfix)[\s.]*\d+([.-]\d+)*\b/ig, '')
    // Collapse dots, then whitespace
    s = s.replace(/\.+/g, ' ')
    s = s.replace(/\s+/g, ' ').trim()
    return s
  }

  function buildGroups(list) {
    const termNameLocal = id => terms.find(t => t.id === id)?.name || ''
    const map = new Map()
    for (const m of list) {
      const raw = (m.item_title || '').trim()
      const key = normalizeTitle(raw)
      if (!key) continue
      let g = map.get(key)
      if (!g) {
        // artQuery is the clean human search-term name (e.g. "Dune Awakening"),
        // NOT the raw torrent filename, so SteamGridDB matches the right game.
        g = { key, title: key, artQuery: termNameLocal(m.search_term_id) || raw, items: [], sourceIds: new Set(), latest: m }
        map.set(key, g)
      }
      g.items.push(m)
      g.sourceIds.add(m.source_id)
      if (new Date(m.matched_at) > new Date(g.latest.matched_at)) g.latest = m
    }
    const arr = [...map.values()]
    for (const g of arr) {
      g.items.sort((a, b) => new Date(b.matched_at) - new Date(a.matched_at))
      // Display title: prefer the most common search-term name among items
      // (user-authored, clean). If no term name is available, fall back to
      // the normalized key (already stripped of version/release-group noise).
      const counts = new Map()
      for (const m of g.items) {
        const n = termNameLocal(m.search_term_id)
        if (n) counts.set(n, (counts.get(n) || 0) + 1)
      }
      if (counts.size) {
        const best = [...counts.entries()].sort((a, b) => b[1] - a[1])[0][0]
        g.title = best
        g.artQuery = best
      } else {
        // Title-case the normalized key for a readable display.
        g.title = g.key.replace(/\b\w/g, c => c.toUpperCase())
        g.artQuery = g.key
      }
    }
    arr.sort((a, b) => new Date(b.latest.matched_at) - new Date(a.latest.matched_at))
    return arr
  }

  // ─── art fetching ──────────────────────────────────────────────────────
  $: fetchArtFor(groups)

  function cacheKey(title) { return (title || '').trim().toLowerCase() }

  function fetchArtFor(gs) {
    let dirty = false
    for (const g of gs) {
      const k = cacheKey(g.artQuery)
      if (artCache.has(k)) continue
      artCache.set(k, 'loading')
      dirty = true
      api.art(g.artQuery)
        .then(r => { artCache.set(k, r?.game || null); artCache = artCache })
        .catch(() => { artCache.set(k, null); artCache = artCache })
    }
    if (dirty) artCache = artCache
  }

  function gameOf(q)     { const v = artCache.get(cacheKey(q)); return (v && v !== 'loading') ? v : null }
  function artStateOf(q) { return artCache.get(cacheKey(q)) ?? 'loading' }

  // ─── click targets ─────────────────────────────────────────────────────
  // Both thumbnail and title navigate to the internal match detail page,
  // which owns Steam button, news, release date, etc (per locked contract).
  function openGroup(group) {
    if (group.latest?.id != null) window.location.hash = `/match/${group.latest.id}`
  }
  function toggle(key) {
    if (expanded.has(key)) expanded.delete(key)
    else expanded.add(key)
    expanded = expanded
  }

  // ─── delete actions ────────────────────────────────────────────────────
  async function deleteGroup(g, ev) {
    ev?.stopPropagation()
    if (!confirm(`Delete ${g.items.length} match${g.items.length === 1 ? '' : 'es'} for "${g.title}"?`)) return
    try {
      await Promise.all(g.items.map(m => api.matches.delete(m.id)))
      matches = matches.filter(m => !g.items.some(gm => gm.id === m.id))
    } catch (e) { error = e.message }
  }
  async function clearAllMatches() {
    if (!confirm(`Delete ALL ${matches.length} matches? This cannot be undone.`)) return
    try {
      await api.matches.clearAll()
      matches = []
    } catch (e) { error = e.message }
  }

  // ─── lookups ───────────────────────────────────────────────────────────
  const termName   = id => terms.find(t => t.id === id)?.name   ?? `#${id}`
  const sourceName = id => sources.find(s => s.id === id)?.name ?? `#${id}`
  const fmt        = dt => dt ? new Date(dt).toLocaleString(undefined, {
    month:'short', day:'numeric', hour:'2-digit', minute:'2-digit'
  }) : '—'
  const initial    = str => (str || '?')[0].toUpperCase()
</script>

<div class="page">
  <div class="page-header">
    <div class="title-block">
      <div class="eyebrow">// watchlist · live feed</div>
      <h1 class="page-title">Dashboard</h1>
    </div>
    <div class="header-right">
      <div class="header-meta">
        <span class="meta-num">{groups.length}</span>
        <span class="meta-label">{groups.length === 1 ? 'title' : 'titles'}</span>
        <span class="meta-sep">·</span>
        <span class="meta-num">{filtered.length}</span>
        <span class="meta-label">hits</span>
      </div>
      <button
        class="btn btn-clear"
        on:click={clearAllMatches}
        disabled={matches.length === 0}
        title="Delete all matches"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/></svg>
        Clear all
      </button>
      <button
        class="btn btn-scan {scanState}"
        on:click={scanNow}
        disabled={scanState === 'scanning'}
      >
        {#if scanState === 'scanning'}
          <svg class="spin" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" stroke-opacity=".25"/><path d="M21 12a9 9 0 00-9-9"/></svg>
          Scanning…
        {:else if scanState === 'done'}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
          {scanResult?.matches_found ?? 0} found
        {:else if scanState === 'error'}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          Failed
        {:else}
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polygon points="5 3 19 12 5 21 5 3"/></svg>
          Scan Now
        {/if}
      </button>
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
        Clear
      </button>
    {/if}
  </div>

  {#if groups.length === 0}
    <div class="empty-card">
      <div class="empty-ornament">◇◈◇</div>
      <div class="empty-title">
        {matches.length === 0 ? 'no signal yet' : 'nothing matches those filters'}
      </div>
      <div class="empty-sub">
        {matches.length === 0 ? 'Run a scan or wait for the scheduled poll.' : 'Try clearing filters above.'}
      </div>
    </div>
  {:else}
    <div class="group-list">
      {#each groups as g, i (g.key)}
        {@const artState = artStateOf(g.artQuery)}
        {@const game     = gameOf(g.artQuery)}
        {@const isOpen   = expanded.has(g.key)}

        <article
          class="row"
          class:open={isOpen}
          style="--stagger: {Math.min(i, 12) * 28}ms"
        >
          <div
            class="art-wrap has-link"
            on:click|stopPropagation={() => openGroup(g)}
            on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && openGroup(g)}
            role="button"
            tabindex="0"
            title="Open match details"
            aria-label={`Open details for ${g.title}`}
          >
            {#if artState === 'loading'}
              <div class="art-shimmer"></div>
            {:else if game?.grid_url}
              <img
                class="art-thumb"
                src={game.grid_url}
                alt={g.title}
                loading="lazy"
                on:error={() => { artCache.set(cacheKey(g.artQuery), null); artCache = artCache }}
              />
            {:else}
              <div class="art-placeholder">{initial(g.title)}</div>
            {/if}
            <span class="art-badge" aria-hidden="true">↗</span>
          </div>

          <div class="main-col">
            <button
              type="button"
              class="title-btn"
              on:click={() => openGroup(g)}
              title="Open match details"
            >
              <span class="title-text">{g.title}</span>
            </button>
            <div class="title-sub mono">{g.items.length} {g.items.length === 1 ? 'hit' : 'hits'} · matched on {g.sourceIds.size} indexer{g.sourceIds.size !== 1 ? 's' : ''}</div>
          </div>

          <div class="stats-col">
            <div class="hit-pill" title={`${g.items.length} total hits across ${g.sourceIds.size} indexer${g.sourceIds.size!==1?'s':''}`}>
              <span class="hit-num">{g.items.length}</span>
              <span class="hit-label">{g.items.length === 1 ? 'hit' : 'hits'}</span>
            </div>
            <div class="sub-stat">
              on <strong>{g.sourceIds.size}</strong> indexer{g.sourceIds.size !== 1 ? 's' : ''}
            </div>
            <div class="sub-stat muted mono">latest · {fmt(g.latest.matched_at)}</div>
          </div>

          <div class="row-actions">
            <button
              type="button"
              class="del-btn"
              on:click={(e) => deleteGroup(g, e)}
              title="Delete these matches"
              aria-label={`Delete ${g.items.length} matches for ${g.title}`}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/>
              </svg>
            </button>
            <button
              type="button"
              class="chev-btn"
              class:open={isOpen}
              on:click={() => toggle(g.key)}
              aria-expanded={isOpen}
              aria-label={isOpen ? 'Collapse matches' : 'Expand matches'}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="6 9 12 15 18 9"/>
              </svg>
            </button>
          </div>

          {#if isOpen}
            <div
              class="drawer"
              transition:slide={{ duration: 220, easing: cubicOut }}
            >
              <div class="drawer-inner">
                <div class="drawer-head">
                  <span class="drawer-label">all matches</span>
                  <span class="drawer-count">{g.items.length}</span>
                </div>
                <table class="sub-table">
                  <thead>
                    <tr>
                      <th>Release</th>
                      <th>Indexer</th>
                      <th>Term</th>
                      <th>Matched</th>
                      <th class="th-link"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each g.items as m}
                      <tr>
                        <td class="release-cell mono" title={m.item_title}>{m.item_title}</td>
                        <td>{sourceName(m.source_id)}</td>
                        <td><span class="term-pill">{termName(m.search_term_id)}</span></td>
                        <td class="mono muted">{fmt(m.matched_at)}</td>
                        <td class="td-link">
                          {#if m.item_url}
                            <a href={m.item_url} target="_blank" rel="noopener noreferrer">release ↗</a>
                          {:else}
                            <span class="muted">—</span>
                          {/if}
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
                <!-- reserved slot for future news snippet from /api/match/:id -->
                <div class="news-slot" aria-hidden="true"></div>
              </div>
            </div>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .title-block { display: flex; flex-direction: column; gap: 0.3rem; }
  .eyebrow {
    font-family: var(--font-mono);
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--accent);
    opacity: 0.75;
  }

  .header-right { display: flex; align-items: center; gap: 0.7rem; }

  .header-meta {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
    font-family: var(--font-mono);
    font-size: 0.78rem;
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    padding: 0.35rem 0.85rem;
    border-radius: 20px;
  }
  .meta-num   { color: var(--text); font-weight: 700; }
  .meta-label { text-transform: uppercase; letter-spacing: 0.08em; font-size: 0.68rem; }
  .meta-sep   { color: var(--text-dim); }

  .btn-scan   { font-weight: 600; letter-spacing: 0.02em; gap: 0.35rem; border-color: var(--border-2); color: var(--text-muted); }
  .btn-scan:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); background: var(--accent-dim); }
  .btn-scan.scanning { border-color: var(--accent); color: var(--accent); background: var(--accent-dim);
                      animation: scan-pulse 1.4s ease-in-out infinite; cursor: wait; }
  .btn-scan.done     { border-color: var(--green); color: var(--green); background: var(--green-dim); }
  .btn-scan.error    { border-color: var(--red);   color: var(--red);   background: var(--red-dim); }
  .btn-clear         { font-weight: 600; letter-spacing: 0.02em; gap: 0.35rem; border-color: var(--border-2); color: var(--text-muted); }
  .btn-clear:hover:not(:disabled) { border-color: var(--red); color: var(--red); background: var(--red-dim); }
  .btn-clear:disabled { opacity: 0.35; cursor: not-allowed; }
  .row-actions { display: flex; gap: 0.4rem; align-items: center; }
  .del-btn {
    width: 32px; height: 32px;
    display: flex; align-items: center; justify-content: center;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .del-btn:hover { color: var(--red); border-color: var(--red); background: var(--red-dim); }
  .spin { animation: spin 0.8s linear infinite; }
  @keyframes scan-pulse { 0%,100% { box-shadow: 0 0 0 0 var(--accent-glow); } 50% { box-shadow: 0 0 14px 4px var(--accent-glow); } }
  @keyframes spin       { to { transform: rotate(360deg); } }

  .filters { display: flex; gap: 0.6rem; align-items: center; }
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

  .group-list { display: flex; flex-direction: column; gap: 0.55rem; }

  /* Each .row is a self-contained card. Hover affordance = border + inner
     shadow + background shift — NO transforms, nothing to clip. We do NOT
     wrap this list in .table-wrap (which sets overflow:hidden globally). */
  .row {
    display: grid;
    grid-template-columns: 72px 1fr auto 38px;
    grid-template-areas:
      "art main stats chev"
      "drawer drawer drawer drawer";
    align-items: center;
    gap: 0 1.1rem;
    padding: 0.7rem 0.95rem;
    background: linear-gradient(180deg, var(--surface) 0%, var(--surface-2) 100%);
    border: 1px solid var(--border);
    border-radius: 12px;
    position: relative;
    transition: border-color 0.18s ease, box-shadow 0.22s ease, background 0.2s ease;
    animation: row-in 0.55s cubic-bezier(.22,.9,.3,1.1) backwards;
    animation-delay: var(--stagger, 0ms);
  }
  .row::before {
    content: '';
    position: absolute;
    left: 0; top: 12px; bottom: 12px;
    width: 2px;
    background: var(--border-2);
    border-radius: 2px;
    transition: background 0.18s ease, box-shadow 0.18s ease;
  }
  .row:hover {
    border-color: var(--accent);
    box-shadow:
      inset 0 0 0 1px rgba(249,115,22,0.25),
      inset 0 0 24px -4px rgba(249,115,22,0.18);
    background: linear-gradient(180deg, var(--surface-2) 0%, var(--surface-3) 100%);
  }
  .row:hover::before { background: var(--accent); box-shadow: 0 0 8px var(--accent-glow); }
  .row.open {
    border-color: var(--accent);
    box-shadow: inset 0 0 0 1px rgba(249,115,22,0.3);
  }
  .row.open::before { background: var(--accent); }

  @keyframes row-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .art-wrap {
    grid-area: art;
    width: 56px;
    height: 74px;
    position: relative;
    border-radius: 5px;
    outline: none;
    cursor: default;
  }
  .art-wrap.has-link { cursor: pointer; }
  .art-thumb, .art-placeholder, .art-shimmer {
    width: 56px;
    height: 74px;
    border-radius: 5px;
    display: block;
  }
  .art-thumb {
    object-fit: cover;
    border: 1px solid var(--border);
    transition: border-color 0.18s ease, box-shadow 0.18s ease, filter 0.18s ease;
  }
  .art-placeholder {
    background: var(--surface-3);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-head);
    font-size: 1.4rem;
    font-weight: 800;
    color: var(--text-dim);
    letter-spacing: -0.02em;
  }
  .art-shimmer {
    background: linear-gradient(90deg, var(--surface-2) 25%, var(--surface-3) 50%, var(--surface-2) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }
  @keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }

  .art-wrap.has-link:hover .art-thumb,
  .art-wrap.has-link:focus-visible .art-thumb,
  .art-wrap.has-link:hover .art-placeholder,
  .art-wrap.has-link:focus-visible .art-placeholder {
    border-color: var(--accent);
    box-shadow:
      inset 0 0 0 1px var(--accent),
      inset 0 0 12px 1px rgba(249,115,22,0.55);
    filter: brightness(1.12) saturate(1.1);
  }
  .art-badge {
    position: absolute;
    top: -4px;
    right: -4px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent);
    color: #0b0b14;
    font-size: 0.7rem;
    font-weight: 800;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transform: scale(0.6);
    transition: opacity 0.18s ease, transform 0.18s ease;
    pointer-events: none;
    box-shadow: 0 2px 8px var(--accent-glow);
  }
  .art-wrap.has-link:hover .art-badge,
  .art-wrap.has-link:focus-visible .art-badge { opacity: 1; transform: scale(1); }

  .main-col {
    grid-area: main;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .title-btn {
    all: unset;
    cursor: pointer;
    font-family: var(--font-head);
    font-size: 1.02rem;
    font-weight: 700;
    color: var(--text);
    letter-spacing: -0.01em;
    line-height: 1.2;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }
  .title-btn .title-text {
    border-bottom: 1px dashed transparent;
    transition: color 0.15s ease, border-color 0.15s ease;
  }
  .title-btn:hover .title-text,
  .title-btn:focus-visible .title-text {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .title-sub {
    font-size: 0.72rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .stats-col {
    grid-area: stats;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.25rem;
    min-width: 150px;
  }
  .hit-pill {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
    background: var(--accent-dim);
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 3px 10px;
    border-radius: 999px;
    font-family: var(--font-mono);
  }
  .hit-num   { font-size: 0.95rem; font-weight: 800; line-height: 1; }
  .hit-label { font-size: 0.64rem; text-transform: uppercase; letter-spacing: 0.1em; }

  .sub-stat  { font-size: 0.75rem; color: var(--text-muted); }
  .sub-stat strong { color: var(--text); font-weight: 700; }
  .sub-stat.mono   { font-family: var(--font-mono); font-size: 0.7rem; }

  .chev-btn {
    grid-area: chev;
    all: unset;
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    color: var(--text-muted);
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--surface);
    transition: color 0.15s ease, border-color 0.15s ease, background 0.15s ease, transform 0.25s ease;
  }
  .chev-btn:hover { color: var(--accent); border-color: var(--accent); background: var(--accent-dim); }
  .chev-btn.open  { color: var(--accent); border-color: var(--accent); background: var(--accent-dim); transform: rotate(180deg); }

  .drawer {
    grid-area: drawer;
    overflow: hidden;
    margin-top: 0.7rem;
  }
  .drawer-inner {
    border-top: 1px dashed var(--border-2);
    padding-top: 0.75rem;
    padding-left: 72px;
  }
  .drawer-head {
    display: flex;
    align-items: baseline;
    gap: 0.55rem;
    margin-bottom: 0.4rem;
  }
  .drawer-label {
    font-family: var(--font-mono);
    font-size: 0.64rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--accent);
    opacity: 0.8;
  }
  .drawer-count {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .sub-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
  }
  .sub-table th {
    text-align: left;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-dim);
    font-weight: 500;
    padding: 0.35rem 0.6rem;
    border-bottom: 1px solid var(--border);
  }
  .sub-table td {
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid var(--border);
    color: var(--text);
  }
  .sub-table tbody tr:last-child td { border-bottom: none; }
  .sub-table tbody tr:hover td      { background: rgba(249,115,22,0.04); }

  .release-cell {
    font-size: 0.74rem;
    color: var(--text);
    max-width: 34ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .th-link, .td-link { text-align: right; width: 1%; white-space: nowrap; }
  .td-link a {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text-muted);
    border-bottom: 1px dashed var(--border-2);
  }
  .td-link a:hover { color: var(--accent); border-bottom-color: var(--accent); text-decoration: none; }

  .term-pill {
    background: var(--accent-dim);
    color: var(--accent);
    padding: 2px 8px;
    border-radius: 20px;
    font-size: 0.72rem;
    font-weight: 600;
    white-space: nowrap;
  }

  .news-slot { min-height: 0; }

  .empty-card {
    border: 1px dashed var(--border-2);
    border-radius: 12px;
    padding: 3.5rem 1rem;
    text-align: center;
    background: repeating-linear-gradient(
      135deg,
      var(--surface) 0 12px,
      var(--surface-2) 12px 24px
    );
  }
  .empty-ornament {
    font-family: var(--font-mono);
    color: var(--accent);
    opacity: 0.5;
    letter-spacing: 0.6em;
    font-size: 0.9rem;
    margin-bottom: 0.7rem;
  }
  .empty-title {
    font-family: var(--font-head);
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--text);
    text-transform: lowercase;
    letter-spacing: -0.01em;
  }
  .empty-sub { color: var(--text-muted); font-size: 0.85rem; margin-top: 0.3rem; }

  .mono  { font-family: var(--font-mono); font-size: 0.8rem; }
  .muted { color: var(--text-muted); }

  @media (max-width: 720px) {
    .header-right { flex-wrap: wrap; gap: 0.5rem; width: 100%; justify-content: flex-start; }
    .header-meta { font-size: 0.72rem; padding: 0.3rem 0.7rem; }
    .filters { flex-wrap: wrap; gap: 0.4rem; }
    .filter-select { flex: 1 1 45%; min-width: 0; }

    .row {
      grid-template-columns: 56px 1fr 34px;
      grid-template-areas:
        "art main chev"
        "art stats chev"
        "drawer drawer drawer";
      gap: 0 0.8rem;
      padding: 0.65rem 0.75rem;
    }
    .stats-col { align-items: flex-start; min-width: 0; }
    .hit-pill { padding: 2px 8px; }
    .drawer-inner { padding-left: 0; }

    /* Sub-table inside drawer → stack into cards on mobile */
    .sub-table,
    .sub-table thead,
    .sub-table tbody,
    .sub-table tr,
    .sub-table td { display: block; width: 100%; }
    .sub-table thead { display: none; }
    .sub-table tr {
      padding: 0.5rem 0;
      border-bottom: 1px dashed var(--border);
    }
    .sub-table tr:last-child { border-bottom: none; }
    .sub-table td { padding: 0.2rem 0; border-bottom: none; }
    .sub-table tr > td:first-child {
      font-family: var(--font-head);
      font-weight: 700;
      color: var(--text);
      font-size: 0.88rem;
      padding-bottom: 0.15rem;
    }
    .th-link, .td-link { text-align: left; width: auto; }
  }

  @media (max-width: 480px) {
    .title-btn { font-size: 0.95rem; white-space: normal; }
    .title-sub { font-size: 0.68rem; }
  }
</style>
