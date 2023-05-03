<script>
    import moment from "moment";
    import { page } from "$app/stores";

    /**
     * @type {{ when: string; confidence: number; }}
     */
    export let file;
</script>

<div class="statistics">
    <div class="when">
        {moment(file.when).fromNow()} ({moment(file.when).format("h:mm:ss a")})
    </div>
    <div class="confidence">
        {file.confidence}
    </div>
</div>

<style>
    .statistics {
        display: flex;
        flex-direction: row;
        justify-content: space-between;
    }
</style>
