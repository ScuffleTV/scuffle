<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";
	import { fade } from "svelte/transition";
	import { isMobile, mouseTrap } from "$/lib/utils";
	import Fa from "svelte-fa";
	import { faXmark } from "@fortawesome/free-solid-svg-icons";

	const dispatch = createEventDispatcher();

	let dialog: HTMLDialogElement;

	export let width: number = 25;
	export let showClose: boolean = true;

	onMount(() => {
		dialog.showModal();
	});

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === "Escape") {
			dispatch("close");
		}
	}

	function onClose() {
		if (!isMobile()) {
			dispatch("close");
		}
	}
</script>

<svelte:window on:keydown={handleKeyDown} />

<dialog
	bind:this={dialog}
	use:mouseTrap={onClose}
	aria-modal="true"
	transition:fade={{ duration: 100 }}
	style="--width-prop: {width}rem"
>
	<div use:mouseTrap={onClose}>
		<slot />
		{#if showClose}
			<button class="close-button" on:click={() => dispatch("close")}>
				<Fa icon={faXmark} />
			</button>
		{/if}
	</div>
</dialog>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	dialog {
		margin: auto;
		border: none;
		padding: 0;
		background: none;

		max-width: 90vw;
		width: var(--width-prop);

		div {
			color: $textColor;
			font-weight: 500;

			width: 100%;
			padding: 2rem;
			border: 1px solid $borderColor;
			border-radius: 0.5rem;
			background-color: rgba($bgColorLight, 0.8);
		}

		backdrop-filter: blur(2rem);

		&::backdrop {
			background-color: rgba(0, 0, 0, 0.5);
		}
	}

	.close-button {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;

		color: $textColorLight;
		font-size: 1.5rem;
	}
</style>
