<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import Dialog from "../dialog.svelte";
	import ImageEditor from "./image-editor.svelte";
	import Fa from "svelte-fa";
	import { faBorderAll, faCheck, faUpRightAndDownLeftFromCenter } from "@fortawesome/free-solid-svg-icons";
	import Spinner from "../spinner.svelte";

    export let src: string;

    const dispatch = createEventDispatcher();

    let loading = false;
    let editor: ImageEditor;
    let scale: number = 1;
    let grid = false;

    function onSubmit() {
        if (!editor) return;
        loading = true;
        editor.calculateResult((blob) => {
            loading = false;
            dispatch("submit", { blob: blob });
        });
    }
</script>

<Dialog width={25} on:close>
    <div class="container">
        <div class="editor">
            <ImageEditor size={20 * 16} minScale={1} maxScale={2} bind:this={editor} bind:scale={scale} src={src} gridOverlay={grid} />
            <div class="input-row">
                <Fa icon={faUpRightAndDownLeftFromCenter} fw />
                <input class="scale" type="range" min="1" max="2" step="0.01" bind:value={scale} />
                <button class="button" class:secondary={!grid} class:primary={grid} on:click={() => (grid = !grid)}>
                    <Fa icon={faBorderAll} fw />
                </button>
            </div>
        </div>
        <div class="buttons">
            <button class="button secondary" on:click={() => dispatch("close")}>Cancel</button>
            <button class="button primary submit" on:click={onSubmit}>
                Submit
                {#if loading}
                    <Spinner />
                {:else}
                    <Fa icon={faCheck} />
                {/if}
            </button>
        </div>
    </div>
</Dialog>

<style lang="scss">
    @import "../../assets/styles/variables.scss";

    .container {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 2rem;
    }

    .editor {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 1rem;

        & > .input-row {
            width: 100%;

            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 1rem;

            color: $textColorLight;

            & > .button {
                padding: 0.5rem 0.6rem;
                display: flex;
                align-items: center;
                justify-content: center;
            }

            & > .scale {
                width: 100%;
            }
        }
    }

    .buttons {
        width: 100%;

        display: flex;
        justify-content: space-between;
        gap: 1rem;

        & > .button {
            padding: 0.4rem 0.8rem;

            &.submit {
                display: flex;
                align-items: center;
                gap: 0.5rem;
            }
        }
    }
</style>
