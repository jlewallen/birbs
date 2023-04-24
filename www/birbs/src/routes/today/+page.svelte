<script>
  import moment from "moment";
  import Today from "./Today.svelte";
  import Lazy from "svelte-lazy";

  /** @type {import('./$types').PageData} */
  export let data;
</script>

<svelte:head>
  <title>Today</title>
</svelte:head>

<div>
  <h1>Today</h1>

  <div class="body">
    {#await data.recently}
      Loading...
    {:then value}
      {#each value.detections as file}
        {#if file.available}
          <div class="detection file">
            <div class="header">
              <div class="row">
                <div class="name">
                  {file.common_name}
                </div>
              </div>
              <div class="row">
                <div class="sub">
                  <div class="when">
                    {moment(file.when).format("dddd, MMMM Do YYYY, h:mm:ss a")}
                  </div>
                  <div class="confidence">
                    {file.confidence}
                  </div>
                </div>
              </div>
            </div>
            <div class="video">
              <Lazy height={600}>
                <!-- svelte-ignore a11y-media-has-caption -->
                <video
                  style="margin-top: 10px"
                  controls
                  poster={file.spectrogram_url}
                  preload="none"
                  title={file.audio_url}><source src={file.audio_url} /></video
                >
              </Lazy>
            </div>
          </div>
        {/if}
      {/each}
    {:catch error}
      {error.message}
    {/await}
  </div>

  <Today />
</div>

<style>
  .detection,
  .file {
    margin-top: 1em;
  }

  .detection .row {
    display: flex;
    flex-direction: column;
  }

  .detection .sub {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: end;
  }

  .detection .header .name {
    font-size: 18pt;
    margin-left: 1em;
    margin-bottom: 0.25em;
  }

  .detection .confidence {
    color: darkgreen;
    font-size: 14px;
    margin-right: 2em;
  }

  .detection .when {
    margin-left: 1.5em;
    font-size: 14pt;
  }

  .detection .video {
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
