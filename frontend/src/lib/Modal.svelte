<script>
  export let title = ''
  export let onClose = () => {}
  export let wide = false
</script>

<div
  class="overlay"
  on:click|self={onClose}
  on:keydown={(e) => { if (e.key === 'Escape') onClose() }}
  role="dialog"
  aria-modal="true"
>
  <div class="modal" class:wide tabindex="-1">
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
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(249,115,22,0.08);
    animation: slide-up 0.15s ease;
  }
  .modal.wide { max-width: 900px; }
  .modal-body { overflow-y: auto; }

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

  /* ───────── Responsive ───────── */
  @media (max-width: 720px) {
    .overlay { padding: 0.75rem; align-items: flex-start; }
    .modal,
    .modal.wide {
      min-width: 0;
      max-width: 100%;
      width: 100%;
      max-height: calc(100vh - 1.5rem);
      max-height: calc(100dvh - 1.5rem);
      border-radius: 10px;
    }
    .modal-header { padding: 1rem 1.1rem; }
    .modal-header h3 { font-size: 1rem; }
    .modal-body { padding: 1rem 1.1rem; gap: 0.85rem; }
    .modal-footer {
      padding: 0.85rem 1.1rem;
      flex-wrap: wrap;
    }
  }
</style>
