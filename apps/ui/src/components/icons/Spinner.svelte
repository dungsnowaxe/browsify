<svg class="loading-spinner" viewBox="0 0 24 24" aria-hidden="true">
  <circle
    cx="12"
    cy="12"
    r="10"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-dasharray="62.83"
    stroke-dashoffset="15.7"
  />
</svg>

<style>
  .loading-spinner {
    animation: browsify-spin 0.9s linear infinite;
  }

  @keyframes browsify-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .loading-spinner {
      animation: none;
    }
  }
</style>
