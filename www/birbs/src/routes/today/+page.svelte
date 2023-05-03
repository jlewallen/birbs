<script>
  import moment from "moment";
  import DetectionFile from "../DetectionFile.svelte";
  import DetectionRow from "../DetectionRow.svelte";

  /** @type {import('./$types').PageData} */
  export let data;
</script>

<svelte:head>
  <title>24h</title>
</svelte:head>

<div>
  <h1>24h</h1>

  <div class="body">
    {#await data.recently}
      Loading...
    {:then value}
      <div class="summarized">
        {#each value.summarized as species}
          <div class="species">
            <div class="name">
              <a href="/birds/{species.common_name}">{species.common_name}</a>
              (<span class="total-24h">{species.total_24h}</span>)
            </div>
            <div class="interesting-detection last">
              <DetectionRow file={species.last} />
              <DetectionFile file={species.last} />
            </div>
            <div class="interesting-detection best">
              <DetectionRow file={species.best} />
              <DetectionFile file={species.best} />
            </div>
          </div>
        {/each}
      </div>
    {:catch error}
      {error.message}
    {/await}
  </div>
</div>

<style>
  .body {
    width: 940px;
  }

  .species {
    margin-bottom: 1em;
  }

  .species .interesting-detection {
    margin-top: 0.5em;
  }
</style>
