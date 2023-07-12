<script lang="ts">
	import { createEventDispatcher } from "svelte";

	const em = createEventDispatcher();

	export let inheritAll = true;
	export let open = false;
	export let openAnimationDuration = 1000;
	export let closeAnimationDuration = 1000;

	let timeout: NodeJS.Timeout;
	let innerOpen = open;
	$: styleState = `
        --open-animation-duration: ${openAnimationDuration + "ms"};
        --close-animation-duration: ${closeAnimationDuration + "ms"};
    `;
	let classState = "";

	$: {
		if (open) {
			clearTimeout(timeout);
			if (!innerOpen) {
				innerOpen = true;
				em("opened");
			}

			timeout = setTimeout(() => {
				classState = "open";
			}, 10); // 10ms so that the page renders and then the animation starts
		} else {
			clearTimeout(timeout);
			classState = "closed";
			timeout = setTimeout(() => {
				innerOpen = false;
				em("closed");
			}, closeAnimationDuration);
		}
	}
</script>

{#if innerOpen}
	<div class={classState + (inheritAll ? " all" : "")} style={styleState}>
		<slot />
	</div>
{/if}

<style lang="scss">
	div {
		opacity: 0;
	}

	.all {
		all: inherit;
	}

	.open {
		transition: opacity var(--open-animation-duration) ease;
		opacity: 1;
	}

	.closed {
		transition: opacity var(--close-animation-duration) ease;
		opacity: 0;
	}
</style>
