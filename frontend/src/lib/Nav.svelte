<script>
  const links = [
    { href: '#/', label: 'Dashboard' },
    { href: '#/search-terms', label: 'Search Terms' },
    { href: '#/sources', label: 'Sources' },
    { href: '#/notifications', label: 'Notifications' },
  ]
</script>

<nav>
  <div class="logo">DiscoProwl</div>
  {#each links as link}
    <a href={link.href}>{link.label}</a>
  {/each}
</nav>

<style>
  nav {
    width: 200px; background: #1a1a2e; color: #eee;
    display: flex; flex-direction: column; padding: 1rem; gap: 0.5rem;
    min-height: 100vh;
  }
  .logo { font-weight: bold; font-size: 1.1rem; margin-bottom: 1rem; color: #7c83fd; }
  a { color: #ccc; text-decoration: none; padding: 0.4rem 0.5rem; border-radius: 4px; }
  a:hover { background: #2a2a4e; color: #fff; }
</style>
