<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";
	import MouseTrap from "./mouse-trap.svelte";
	import { fade } from "svelte/transition";

	const dispatch = createEventDispatcher();

	let dialog: HTMLDialogElement;

	export let width: number = 30;

	onMount(() => {
		dialog.showModal();
	});

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			dispatch("close");
		}
	}
</script>

<svelte:window on:keydown={handleKeyDown} />

<dialog
	bind:this={dialog}
	aria-modal="true"
	transition:fade={{ duration: 100 }}
	style="width: min({width}rem, 90vw);"
>
	<MouseTrap on:close={() => dispatch("close")}>
		<slot />
	</MouseTrap>
</dialog>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	dialog {
		background: linear-gradient(to bottom, #18191a, #101415);
		color: $textColor;
		box-shadow: 0 0 0.5rem 0.5rem rgba(0, 0, 0, 0.3);

		padding: 2.5rem;
		border: none;
		border-radius: 0.25rem;
		font-weight: 500;

		&::backdrop {
			background-color: rgba(0, 0, 0, 0.5);
		}
	}
</style>
