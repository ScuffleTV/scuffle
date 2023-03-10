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

	function close(event: MouseEvent) {
		if (!ready) return;

		if (!el.contains(event.target as Node) || event.target === el) {
			em("close");
		}
	}
</script>

<div bind:this={el} class={inheritAll ? "all" : ""}>
	<slot />
</div>

<svelte:window on:click={close} />

<style lang="scss">
	.all {
		all: inherit;
	}
</style>
