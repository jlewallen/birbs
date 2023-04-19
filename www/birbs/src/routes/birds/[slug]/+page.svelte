<script>
  /** @type {import('./$types').PageData} */
  export let data;
</script>

<svelte:head>
  <title>{data.bird.common_name}</title>
</svelte:head>

<section>
  <div class="details-row">
    <div class="half">
      <img src={data.bird.photo_url} alt="A bird" />
    </div>
    <div class="half">
      <h1>{data.bird.common_name}</h1>
    </div>
  </div>

  <div class="body">
    {#await data.files}
      Loading...
    {:then value}
      {#each data.files as file}
        {#if file.available}
          <div>
            {file.when}
            {file.confidence}

            <!-- svelte-ignore a11y-media-has-caption -->
            <video
              style="margin-top: 10px"
              controls
              poster={file.spectrogram_url}
              preload="none"
              title={file.audio_url}><source src={file.audio_url} /></video
            >
          </div>
        {/if}
      {/each}
    {:catch error}
      {error.message}
    {/await}
  </div>
</section>

<style>
  .details-row {
    display: flex;
    flex-direction: row;
    background-color: white;
    padding: 1em;
    margin-bottom: 1em;
    border: 1px solid black;
  }

  .details-row img {
    border: 1px solid black;
    object-fit: scale-down;
    align-items: center;
  }

  .details-row h1 {
    margin: 0em 0em 0em 1em;
    text-align: left;
  }
</style>
