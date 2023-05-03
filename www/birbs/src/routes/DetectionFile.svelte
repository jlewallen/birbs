<script>
    import { page } from "$app/stores";

    /**
     * @type {{ spectrogram_url: string; audio_url: string; }}
     */
    export let file;
</script>

<div class="video">
    <!-- svelte-ignore a11y-media-has-caption -->
    <video
        style="margin-top: 10px"
        controls
        poster={file.spectrogram_url}
        preload="none"
        title={file.audio_url}><source src={file.audio_url} /></video
    >
</div>

<style>
    .video {
        display: flex;
        align-items: center;
        justify-content: center;
    }
</style>
