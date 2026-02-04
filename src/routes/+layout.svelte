<script lang="ts">
  import "../app.css";

  let { children } = $props();
</script>

<div class="min-h-screen bg-sidecar-bg">
  {@render children()}
</div>
