<script lang="ts">
    import Dialog from "$/components/dialog.svelte";
    import Fa from "svelte-fa";
	import { faExclamationTriangle } from "@fortawesome/free-solid-svg-icons";

    export let heading: string;
    export let message: string;
</script>

<Dialog on:close>
    <div class="heading">
        <Fa icon={faExclamationTriangle} />
        <h1>{heading}</h1>
    </div>
    <span>{message}</span>
</Dialog>

<style lang="scss">
    .heading {
        display: flex;
        align-items: center;
        gap: 0.5rem;

        margin-bottom: 1rem;
        font-size: 1.5rem;
    }

    h1 {
        font-size: 1.5rem;
        font-weight: 500;
    }

    span {
        font-size: 1rem;
    }
</style>
