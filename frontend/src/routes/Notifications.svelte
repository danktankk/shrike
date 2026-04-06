<script>
  import { onMount } from 'svelte'
  import { api } from '../lib/api.js'

  let config = null
  let error = null
  let testResults = {}

  onMount(async () => {
    try { config = await api.notifications.getConfig() }
    catch(e) { error = e.message }
  })

  async function test(channel) {
    testResults = { ...testResults, [channel]: { status: 'pending', msg: 'Sending...' } }
    try {
      await api.notifications.test(channel)
      testResults = { ...testResults, [channel]: { status: 'ok', msg: 'Sent successfully' } }
    } catch(e) {
      testResults = { ...testResults, [channel]: { status: 'err', msg: e.message } }
    }
  }

  $: channels = config ? [
    {
      key: 'discord',
      label: 'Discord',
      icon: '◈',
      configured: !!config.discord_webhook_url,
      detail: config.discord_webhook_url,
      testable: true,
    },
    {
      key: 'apprise',
      label: 'Apprise',
      icon: '◎',
      configured: !!config.apprise_url,
      detail: config.apprise_url,
      testable: true,
    },
    {
      key: 'pushover',
      label: 'Pushover',
      icon: '◆',
      configured: !!config.pushover_configured,
      detail: null,
      testable: true,
    },
    {
      key: 'steamgriddb',
      label: 'SteamGridDB',
      icon: '⬡',
      configured: !!config.steamgriddb_configured,
      detail: null,
      testable: false,
      note: config.steamgriddb_configured ? null : 'Fallback placeholder image will be used',
    },
  ] : []
</script>

<div class="page">
  <div class="page-header">
    <h1 class="page-title">Notifications</h1>
  </div>

  {#if error}<p class="error-msg">{error}</p>{/if}

  <p class="note">Channels are configured via environment variables. Restart the container to apply changes.</p>

  {#if config}
    <div class="channels-grid">
      {#each channels as ch}
        <div class="channel-card" class:configured={ch.configured}>
          <div class="card-top">
            <div class="card-title">
              <span class="ch-icon">{ch.icon}</span>
              <span>{ch.label}</span>
            </div>
            <div class="card-status" class:on={ch.configured}>
              <span class="status-dot {ch.configured ? 'on' : 'off'}"></span>
              {ch.configured ? 'Configured' : 'Not configured'}
            </div>
          </div>

          {#if ch.detail}
            <div class="card-detail">{ch.detail}</div>
          {/if}

          {#if ch.note}
            <div class="card-note">{ch.note}</div>
          {/if}

          {#if ch.configured && ch.testable}
            <div class="card-actions">
              <button class="btn" on:click={() => test(ch.key)}>Send Test</button>
              {#if testResults[ch.key]}
                <span class="test-result {testResults[ch.key].status}">
                  {testResults[ch.key].msg}
                </span>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .note {
    color: var(--text-muted);
    font-size: 0.85rem;
    margin: 0;
  }

  .channels-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }

  .channel-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    transition: border-color 0.15s;
  }

  .channel-card.configured {
    border-color: rgba(249, 115, 22, 0.25);
    background: var(--surface);
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-family: var(--font-head);
    font-size: 1rem;
    font-weight: 700;
    color: var(--text);
  }

  .ch-icon {
    color: var(--accent);
    font-size: 1rem;
  }

  .card-status {
    font-size: 0.78rem;
    font-weight: 500;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .card-status.on { color: var(--green); }

  .card-detail {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--text-muted);
    background: var(--surface-2);
    padding: 0.4rem 0.6rem;
    border-radius: 5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-note {
    font-size: 0.78rem;
    color: var(--text-dim);
    font-style: italic;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.25rem;
  }

  .test-result {
    font-size: 0.8rem;
    font-family: var(--font-mono);
  }
  .test-result.ok      { color: var(--green); }
  .test-result.err     { color: var(--red); }
  .test-result.pending { color: var(--text-muted); }
</style>
