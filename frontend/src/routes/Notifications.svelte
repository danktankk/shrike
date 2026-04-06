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
    testResults = { ...testResults, [channel]: 'Sending...' }
    try {
      await api.notifications.test(channel)
      testResults = { ...testResults, [channel]: '✓ Sent' }
    } catch(e) {
      testResults = { ...testResults, [channel]: `✗ ${e.message}` }
    }
  }
</script>

<div>
  <h2>Notifications</h2>
  {#if error}<p class="error">{error}</p>{/if}

  {#if config}
    <p class="note">Notification channels are configured via environment variables.
    Restart the container to change them.</p>

    <div class="channels">
      <div class="channel" class:configured={config.discord_webhook_url}>
        <div class="channel-header">
          <span class="channel-name">Discord</span>
          <span class="status">{config.discord_webhook_url ? '● Configured' : '○ Not configured'}</span>
        </div>
        {#if config.discord_webhook_url}
          <p class="masked">URL: {config.discord_webhook_url}</p>
          <button on:click={() => test('discord')}>Send Test</button>
          {#if testResults.discord}<span class="result">{testResults.discord}</span>{/if}
        {/if}
      </div>

      <div class="channel" class:configured={config.apprise_url}>
        <div class="channel-header">
          <span class="channel-name">Apprise</span>
          <span class="status">{config.apprise_url ? '● Configured' : '○ Not configured'}</span>
        </div>
        {#if config.apprise_url}
          <p class="masked">URL: {config.apprise_url}</p>
          <button on:click={() => test('apprise')}>Send Test</button>
          {#if testResults.apprise}<span class="result">{testResults.apprise}</span>{/if}
        {/if}
      </div>

      <div class="channel" class:configured={config.pushover_configured}>
        <div class="channel-header">
          <span class="channel-name">Pushover</span>
          <span class="status">{config.pushover_configured ? '● Configured' : '○ Not configured'}</span>
        </div>
        {#if config.pushover_configured}
          <button on:click={() => test('pushover')}>Send Test</button>
          {#if testResults.pushover}<span class="result">{testResults.pushover}</span>{/if}
        {/if}
      </div>

      <div class="channel" class:configured={config.steamgriddb_configured}>
        <div class="channel-header">
          <span class="channel-name">SteamGridDB (box art)</span>
          <span class="status">{config.steamgriddb_configured ? '● Configured' : '○ Not configured (fallback image used)'}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  h2 { margin-top: 0; }
  .note { color: #666; font-size: 0.9rem; margin-bottom: 1.5rem; }
  .channels { display: flex; flex-direction: column; gap: 1rem; }
  .channel { border: 1px solid #ddd; border-radius: 6px; padding: 1rem; }
  .channel.configured { border-color: #4caf50; }
  .channel-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.5rem; }
  .channel-name { font-weight: bold; }
  .status { font-size: 0.85rem; color: #888; }
  .channel.configured .status { color: #4caf50; }
  .masked { font-size: 0.82rem; color: #666; margin: 0.3rem 0; font-family: monospace; }
  button { padding: 0.3rem 0.8rem; cursor: pointer; }
  .result { margin-left: 0.5rem; font-size: 0.85rem; }
  .error { color: red; }
</style>
