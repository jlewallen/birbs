<script>
  /** @type {import('./$types').PageData} */
  export let data;
</script>

<svelte:head>
  <title>Birbs</title>
  <meta name="description" content="Birbs" />
</svelte:head>

<section>
  {#await data.query}
    Loading...
  {:then value}
    {#each data.query as day}
      <div>
        <h4>{day.date.toDateString()}</h4>
        <div>
          <img
            src={day.histogram_url}
            alt="Histogram of daily bird activity. Shame on me for not writing a meaningful toString or something to list the top values. At any rate, the table below will show the same information."
          />
        </div>
        <div class="birds">
          {#each day.birds as bird}
            <div class="bird">
              <span class="common-name">
                <a href="/birds/{bird.common_name}">{bird.common_name}</a>
                ({bird.total} / {bird.average_confidence.display}) |
              </span>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  {:catch error}
    {error.message}
  {/await}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    flex: 0.6;
  }

  .birds .bird {
    display: inline-block;
    margin: 0.25em;
  }
</style>
