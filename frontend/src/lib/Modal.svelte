<script>
  export let title = ''
  export let onClose = () => {}
</script>

<div class="overlay" on:click|self={onClose} role="dialog" aria-modal="true">
  <div class="modal">
    <div class="modal-header">
      <h3>{title}</h3>
      <button class="close-btn" on:click={onClose} aria-label="Close">✕</button>
    </div>
    <div class="modal-body">
      <slot />
    </div>
    <div class="modal-footer">
      <slot name="footer" />
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    animation: fade-in 0.15s ease;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .modal {
    background: var(--surface);
    border: 1px solid var(--border-2);
    border-radius: 12px;
    min-width: 440px;
    max-width: 560px;
    width: 100%;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(249,115,22,0.08);
    animation: slide-up 0.15s ease;
  }

  @keyframes slide-up {
    from { transform: translateY(12px); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h3 {
    font-family: var(--font-head);
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--text);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 1rem;
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    transition: all 0.1s;
  }
  .close-btn:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .modal-body {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .modal-footer {
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .modal-footer:empty {
    display: none;
  }
</style>
