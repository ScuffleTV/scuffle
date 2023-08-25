<script lang="ts">
	import { createEventDispatcher } from "svelte";

	export let title = "";
	export let details = "";
	export let showReset = false;

	const dispatch = createEventDispatcher();
</script>

<section aria-label={title}>
	<div class="title-bar">
		<h2>{title}</h2>
		{#if showReset}
			<button on:click={() => dispatch("reset")}>Reset</button>
		{/if}
	</div>
	<span class="details">{details}</span>
	<slot />
</section>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	section {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.title-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;

		& > h2 {
			font-size: 1.2rem;
			font-weight: 500;
		}

		& > button {
			color: $textColor;
			font-weight: 500;

			&:hover,
			&:focus-visible {
				text-decoration: underline;
			}
		}
	}

	.details {
		font-size: 1rem;
		font-weight: 500;
		color: $textColorLight;
	}
</style>
