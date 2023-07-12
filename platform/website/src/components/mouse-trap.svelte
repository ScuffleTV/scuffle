<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";

	const em = createEventDispatcher();

	export let inheritAll = true;

	let el: HTMLElement;

	let ready = false;
	onMount(() => {
		setTimeout(() => {
			ready = true;
		}, 10);
	});

	let willClose = false;

	// These functions basically make it so that if the user clicks outside of the element, it will close.
	// If they drag from inside to outside, it will not close.
	// Or if they drag from outside to inside, it will not close.
	// Only when the down and up events are both outside of the element will it close.
	function mouseDown(event: MouseEvent) {
		willClose = ready && (event.target === el || !el.contains(event.target as Node));
	}

	function mouseUp(event: MouseEvent) {
		willClose = willClose && ready && (event.target === el || !el.contains(event.target as Node));
		if (willClose) {
			em("close");
		}
	}
</script>

<div bind:this={el} class:all={inheritAll}>
	<slot />
</div>

<svelte:window on:mousedown={mouseDown} on:mouseup={mouseUp} />

<style lang="scss">
	.all {
		all: inherit;
	}
</style>
