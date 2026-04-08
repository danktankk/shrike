<script>
  import { api } from '../lib/api.js'

  export let id

  let data = null
  let error = null
  let loading = true

  $: load(id)

  async function load(matchId) {
    if (matchId == null) return
    loading = true
    error = null
    data = null
    try {
      data = await api.matches.get(matchId)
    } catch (e) {
      error = e.message
    } finally {
      loading = false
    }
  }

  function back() { window.location.hash = '/' }

  // Response shape (from src/api/art.rs::MatchDetailResponse):
  //   {
  //     match, search_term: {id,name}, source: {id,name,source_type},
  //     game: {
  //       art:  { hero_url, grid_url, logo_url } | null,
  //       info: { steam_appid, store_url, release_date, short_description,
  //               platforms, developers, publishers, genres,
  //               metacritic_score, header_image } | null,
  //       news: [ { title, url, contents, date } ]
  //     } | null
  //   }
  // Match.notification_channels is Option<String> — a JSON-encoded array.

  const fmtDate = dt => dt ? new Date(dt).toLocaleDateString(undefined, { year:'numeric', month:'long', day:'numeric' }) : null
  const fmtFull = dt => dt ? new Date(dt).toLocaleString() : '—'
  const parseChannels = json => {
    try { const v = JSON.parse(json || '[]'); return Array.isArray(v) ? v : [] }
    catch { return [] }
  }
  const join = arr => (Array.isArray(arr) && arr.length) ? arr.join(' · ') : null
  // Steam news contents is BBCode-ish; strip tags for a clean snippet.
  const stripTags = s => (s || '')
    .replace(/\[\/?[^\]]+\]/g, '')    // [b]..[/b], [url=..]..[/url]
    .replace(/<[^>]+>/g, '')          // html
    .replace(/\s+/g, ' ')
    .trim()
  const snippet = (s, n = 240) => {
    const t = stripTags(s)
    return t.length > n ? t.slice(0, n).replace(/\s+\S*$/, '') + '…' : t
  }
  // Metacritic banding
  const metaBand = n =>
    n == null ? '' : n >= 75 ? 'meta-green' : n >= 50 ? 'meta-amber' : 'meta-red'
</script>

<div class="page">
  <button class="back-btn" on:click={back} aria-label="Back to dashboard">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
    <span>Dashboard</span>
  </button>

  {#if loading}
    <div class="status-block">Loading…</div>
  {:else if error}
    <p class="error-msg">{error}</p>
  {:else if data}
    {@const m = data.match}
    {@const g = data.game}
    {@const term = data.search_term}
    {@const src = data.source}
    {@const art = g?.art}
    {@const info = g?.info}
    {@const news = g?.news ?? []}
    {@const hero = art?.hero_url ?? info?.header_image}
    {@const grid = art?.grid_url}
    {@const logo = art?.logo_url}
    {@const channels = parseChannels(m.notification_channels)}
    {@const developers = join(info?.developers)}
    {@const publishers = join(info?.publishers)}
    {@const genres = join(info?.genres)}

    <article class="detail">
      <header
        class="hero"
        class:no-hero={!hero}
        style={hero ? `background-image: url('${hero}')` : ''}
      >
        <div class="hero-scrim"></div>
        <div class="hero-inner">
          {#if grid}
            <img class="grid-art" src={grid} alt="" />
          {/if}
          <div class="hero-text">
            {#if logo}
              <img class="logo-art" src={logo} alt={m.item_title} />
            {:else}
              <h1 class="title">{m.item_title}</h1>
            {/if}
            <div class="meta-row">
              {#if info?.release_date}
                <span class="meta-pill">{fmtDate(info.release_date) ?? info.release_date}</span>
              {/if}
              {#if info?.metacritic_score != null}
                <span class="meta-pill metacritic {metaBand(info.metacritic_score)}">MC {info.metacritic_score}</span>
              {/if}
              {#if info?.platforms?.length}
                {#each info.platforms as p}
                  <span class="meta-pill muted-pill">{p}</span>
                {/each}
              {/if}
              {#if info?.store_url}
                <a class="meta-pill store-link" href={info.store_url} target="_blank" rel="noopener">Steam ↗</a>
              {/if}
            </div>
          </div>
        </div>
      </header>

      {#if info?.short_description || developers || publishers || genres}
        <section class="panel about-panel">
          <h2 class="panel-title">About</h2>
          {#if info?.short_description}
            <p class="summary">{info.short_description}</p>
          {/if}
          <dl class="kv">
            {#if developers}
              <dt>Developer</dt><dd>{developers}</dd>
            {/if}
            {#if publishers}
              <dt>Publisher</dt><dd>{publishers}</dd>
            {/if}
            {#if genres}
              <dt>Genres</dt><dd>{genres}</dd>
            {/if}
          </dl>
        </section>
      {/if}

      <section class="panel">
        <h2 class="panel-title">Matched release</h2>
        <dl class="kv">
          <dt>Item</dt>
          <dd>
            {#if m.item_url}
              <a href={m.item_url} target="_blank" rel="noopener">{m.item_title} <span class="ext">↗</span></a>
            {:else}
              {m.item_title}
            {/if}
          </dd>

          <dt>Search term</dt>
          <dd>{term?.name ?? '—'}</dd>

          <dt>Source</dt>
          <dd>
            {#if src}
              {src.name}
              <span class="src-type">{src.source_type}</span>
            {:else}
              —
            {/if}
          </dd>

          <dt>Matched at</dt>
          <dd class="mono">{fmtFull(m.matched_at)}</dd>

          <dt>Channels</dt>
          <dd class="mono">
            {#if channels.length}
              {channels.join(', ')}
            {:else}
              —
            {/if}
          </dd>
        </dl>
      </section>

      {#if news.length}
        <section class="panel">
          <h2 class="panel-title">Latest news</h2>
          <ul class="news-list">
            {#each news as n}
              <li class="news-item">
                <a class="news-link" href={n.url} target="_blank" rel="noopener">
                  <div class="news-title">{n.title}</div>
                  {#if n.contents}
                    <p class="news-snippet">{snippet(n.contents)}</p>
                  {/if}
                  {#if n.date}
                    <div class="news-date mono">{fmtFull(n.date)}</div>
                  {/if}
                </a>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if !g}
        <p class="note">No Steam / SteamGridDB match for this title.</p>
      {/if}
    </article>
  {/if}
</div>

<style>
  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.75rem 0.4rem 0.55rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-muted);
    font-family: var(--font-body);
    font-size: 0.82rem;
    cursor: pointer;
    transition: all 0.15s ease;
    align-self: flex-start;
  }
  .back-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-dim);
  }

  .status-block {
    padding: 3rem;
    text-align: center;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 0.9rem;
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 1.75rem;
  }

  /* Hero ---------------------------------------------------- */
  .hero {
    position: relative;
    border-radius: 14px;
    overflow: hidden;
    min-height: 340px;
    background-size: cover;
    background-position: center;
    background-color: var(--surface-2);
    border: 1px solid var(--border);
    isolation: isolate;
  }
  .hero.no-hero {
    background-image:
      radial-gradient(1200px 400px at 20% 10%, rgba(249,115,22,0.18), transparent 60%),
      radial-gradient(900px 300px at 90% 90%, rgba(96,165,250,0.12), transparent 60%);
  }
  .hero-scrim {
    position: absolute; inset: 0;
    background:
      linear-gradient(180deg, rgba(7,7,15,0.15) 0%, rgba(7,7,15,0.55) 45%, rgba(7,7,15,0.95) 100%);
    z-index: 1;
  }
  .hero-inner {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: flex-end;
    gap: 1.5rem;
    padding: 2rem 2rem 1.75rem;
    min-height: 340px;
  }
  .grid-art {
    width: 148px;
    height: 198px;
    object-fit: cover;
    border-radius: 8px;
    border: 1px solid var(--border-2);
    box-shadow: 0 18px 48px rgba(0,0,0,0.55);
    flex-shrink: 0;
  }
  .hero-text {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    min-width: 0;
  }
  .logo-art {
    max-width: 420px;
    max-height: 110px;
    width: auto;
    height: auto;
    filter: drop-shadow(0 6px 18px rgba(0,0,0,0.6));
  }
  .title {
    font-family: var(--font-head);
    font-size: 2.4rem;
    font-weight: 800;
    letter-spacing: -0.025em;
    line-height: 1.05;
    color: var(--text);
    text-shadow: 0 2px 24px rgba(0,0,0,0.7);
    margin: 0;
  }
  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }
  .meta-pill {
    display: inline-flex;
    align-items: center;
    padding: 4px 11px;
    background: rgba(249, 115, 22, 0.18);
    border: 1px solid rgba(249, 115, 22, 0.4);
    color: var(--accent);
    border-radius: 20px;
    font-size: 0.76rem;
    font-weight: 600;
    font-family: var(--font-mono);
    letter-spacing: 0.02em;
    backdrop-filter: blur(6px);
  }
  .meta-pill.muted-pill {
    background: rgba(255,255,255,0.06);
    border-color: rgba(255,255,255,0.14);
    color: var(--text);
  }
  .meta-pill.metacritic.meta-green {
    background: rgba(34, 197, 94, 0.18);
    border-color: rgba(34, 197, 94, 0.45);
    color: var(--green);
  }
  .meta-pill.metacritic.meta-amber {
    background: rgba(250, 204, 21, 0.18);
    border-color: rgba(250, 204, 21, 0.45);
    color: #fde047;
  }
  .meta-pill.metacritic.meta-red {
    background: rgba(239, 68, 68, 0.18);
    border-color: rgba(239, 68, 68, 0.45);
    color: var(--red);
  }
  .meta-pill.store-link {
    text-decoration: none;
    background: rgba(96, 165, 250, 0.18);
    border-color: rgba(96, 165, 250, 0.45);
    color: var(--blue);
  }
  .meta-pill.store-link:hover {
    background: rgba(96, 165, 250, 0.3);
    text-decoration: none;
  }

  .news-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .news-item { border-top: 1px solid var(--border); padding-top: 0.75rem; }
  .news-item:first-child { border-top: none; padding-top: 0; }
  .news-link { display: block; text-decoration: none; color: inherit; }
  .news-link:hover .news-title { color: var(--accent); }
  .news-title {
    font-family: var(--font-head);
    font-size: 0.98rem;
    font-weight: 700;
    color: var(--text);
    transition: color 0.15s ease;
    margin-bottom: 0.3rem;
  }
  .news-snippet {
    margin: 0 0 0.35rem;
    color: var(--text-muted);
    font-size: 0.85rem;
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .news-date { color: var(--text-dim); font-size: 0.75rem; }

  /* Panel --------------------------------------------------- */
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1.25rem 1.5rem 1.4rem;
  }
  .panel-title {
    font-family: var(--font-head);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-muted);
    margin: 0 0 1rem;
  }

  .about-panel .summary {
    margin: 0 0 1.1rem;
    font-size: 0.96rem;
    line-height: 1.65;
    color: var(--text);
    max-width: 72ch;
  }

  .kv {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 0.55rem 1.25rem;
    margin: 0;
  }
  .kv dt {
    color: var(--text-muted);
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding-top: 2px;
  }
  .kv dd {
    margin: 0;
    color: var(--text);
    font-size: 0.92rem;
    word-break: break-word;
  }
  .kv dd .ext { font-size: 0.8em; opacity: 0.7; }

  .src-type {
    display: inline-block;
    margin-left: 0.5rem;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--accent);
    background: var(--accent-dim);
    padding: 1px 7px;
    border-radius: 20px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .mono { font-family: var(--font-mono); font-size: 0.85rem; color: var(--text-muted); }

  .note {
    color: var(--text-muted);
    font-style: italic;
    font-size: 0.85rem;
    padding: 0 0.25rem;
  }

  @media (max-width: 720px) {
    .hero { min-height: 0; }
    .hero-inner { flex-direction: column; align-items: flex-start; padding: 1.25rem; min-height: 0; gap: 1.1rem; }
    .grid-art { width: 110px; height: 148px; }
    .title { font-size: 1.6rem; }
    .logo-art { max-width: 100%; max-height: 90px; }
    .panel { padding: 1rem 1.1rem 1.1rem; }
    .about-panel .summary { font-size: 0.9rem; }
    .kv { grid-template-columns: 1fr; gap: 0.15rem 0; }
    .kv dt { padding-top: 0.6rem; }
  }
</style>
