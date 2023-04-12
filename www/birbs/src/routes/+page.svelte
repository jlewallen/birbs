<script>
  import Counter from "./Counter.svelte";

  /** @type {import('./$types').PageData} */
  export let data;
</script>

<svelte:head>
  <title>Birbs</title>
  <meta name="description" content="Birbs" />
</svelte:head>

<section>
  <h1>Birbs</h1>

  {#await data.query}
    Loading...
  {:then value}
    {#each data.query as day}
      <div>
        <h4>{day.date.toDateString()}</h4>
        <div class="birds">
          {#each day.birds as bird}
            <div class="bird">
              <span class="common-name">
                <a href="#">{bird.common_name}</a> ({bird.total} / {bird
                  .average_confidence.display}) |
              </span>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  {:catch error}
    {error.message}
  {/await}

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

  .birds .bird {
    display: inline-block;
    margin: 0.25em;
  }
</style>
