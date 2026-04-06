<script>
  export let label = ''
  export let hint = ''
</script>

<div class="field">
  <label>
    <span class="label-text">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
    <slot />
  </label>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .label-text {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-dim);
    margin-top: -0.2rem;
  }

  :global(.field input:not([type="checkbox"]):not(.toggle)),
  :global(.field select),
  :global(.field textarea) {
    padding: 0.55rem 0.75rem;
    background: var(--surface-2);
    border: 1px solid var(--border-2);
    border-radius: 7px;
    color: var(--text);
    font-family: var(--font-body);
    font-size: 0.875rem;
    transition: border-color 0.15s;
    outline: none;
  }

  :global(.field input:not([type="checkbox"]):not(.toggle):focus),
  :global(.field select:focus) {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-dim);
  }

  :global(.field select) {
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath fill='%235a5a80' d='M1 1l5 5 5-5'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    padding-right: 2rem;
  }
</style>
