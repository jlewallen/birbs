<script>
  import Counter from "./Counter.svelte";
  // import welcome from "$lib/images/svelte-welcome.webp";
  // import welcome_fallback from "$lib/images/svelte-welcome.png";
</script>

<svelte:head>
  <title>Birbs</title>
  <meta name="description" content="Birbs" />
</svelte:head>

<section>
  <h1>Birbs</h1>

  <Counter />
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    flex: 0.6;
  }

  h1 {
    width: 100%;
  }
</style>
