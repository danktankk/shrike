<script>
  export let route = '/'

  const links = [
    { href: '#/',              path: '/',              label: 'Dashboard',     icon: '⬡' },
    { href: '#/search-terms',  path: '/search-terms',  label: 'Search Terms',  icon: '◈' },
    { href: '#/sources',       path: '/sources',       label: 'Sources',       icon: '◎' },
    { href: '#/notifications', path: '/notifications', label: 'Notifications', icon: '◆' },
  ]
</script>

<nav>
  <div class="logo">
    <span class="logo-icon">◈</span>
    <span class="logo-text">Sh<em>rike</em></span>
  </div>

  <ul>
    {#each links as link}
      <li class:active={route === link.path}>
        <a href={link.href}>
          <span class="icon">{link.icon}</span>
          <span>{link.label}</span>
        </a>
      </li>
    {/each}
  </ul>
</nav>

<style>
  nav {
    width: 220px;
    min-width: 220px;
    background: var(--surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 1.5rem 0;
    min-height: 100vh;
    position: sticky;
    top: 0;
    height: 100vh;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0 1.25rem 1.75rem;
    border-bottom: 1px solid var(--border);
    margin-bottom: 1rem;
  }

  .logo-icon {
    color: var(--accent);
    font-size: 1.4rem;
    line-height: 1;
  }

  .logo-text {
    font-family: var(--font-head);
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .logo-text em {
    font-style: normal;
    color: var(--accent);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  li a {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.55rem 0.75rem;
    border-radius: 7px;
    color: var(--text-muted);
    font-size: 0.875rem;
    font-weight: 500;
    text-decoration: none;
    transition: all 0.15s ease;
    border: 1px solid transparent;
  }

  li a:hover {
    color: var(--text);
    background: var(--surface-2);
    text-decoration: none;
  }

  li.active a {
    color: var(--accent);
    background: var(--accent-dim);
    border-color: rgba(249, 115, 22, 0.2);
    font-weight: 600;
  }

  .icon {
    font-size: 0.9rem;
    width: 18px;
    text-align: center;
    flex-shrink: 0;
  }

  /* ───────── Responsive ───────── */
  @media (max-width: 720px) {
    nav {
      width: 100%;
      min-width: 0;
      min-height: 0;
      height: auto;
      position: sticky;
      top: 0;
      z-index: 50;
      padding: 0.6rem 0.85rem 0.55rem;
      border-right: none;
      border-bottom: 1px solid var(--border);
      display: flex;
      flex-direction: column;
      gap: 0.55rem;
      backdrop-filter: blur(8px);
      background: rgba(14, 14, 28, 0.92);
    }
    .logo {
      padding: 0 0.25rem 0.5rem;
      margin-bottom: 0;
      border-bottom: none;
    }
    ul {
      padding: 0;
      flex-direction: row;
      gap: 0.35rem;
      overflow-x: auto;
      scrollbar-width: none;
    }
    ul::-webkit-scrollbar { display: none; }
    li { flex: 1 0 auto; }
    li a {
      padding: 0.7rem 0.85rem;
      font-size: 0.8rem;
      justify-content: center;
      white-space: nowrap;
      min-height: 44px;
    }
  }

  @media (max-width: 480px) {
    li a { padding: 0.7rem 0.6rem; gap: 0.4rem; font-size: 0.74rem; }
    .logo-text { font-size: 1rem; }
  }
</style>
