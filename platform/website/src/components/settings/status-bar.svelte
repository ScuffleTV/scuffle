<script context="module" lang="ts">
	export enum Status {
		Unchanged,
		Changed,
		Saving,
	}
</script>

<script lang="ts">
	import Fa from "svelte-fa";
	import { faCheck, faSave, faWarning } from "@fortawesome/free-solid-svg-icons";
	import Spinner from "../spinner.svelte";
	import { createEventDispatcher } from "svelte";
	import { beforeNavigate } from "$app/navigation";

	export let status: Status;
	export let saveDisabled = false;

	const dispatch = createEventDispatcher();

	let highlighted = false;

	$: status, (highlighted = false);

	beforeNavigate((nav) => {
		if (status !== Status.Unchanged && !highlighted) {
			highlighted = true;
			nav.cancel();
		}
	});
</script>

<div
	class="bar"
	class:shown={status === Status.Changed || status === Status.Saving}
	class:highlighted
>
	<span class="status">
		{#if status === Status.Saving}
			<Spinner />
			Saving
		{:else if status === Status.Unchanged}
			<Fa icon={faCheck} />
			Saved
		{:else if status === Status.Changed}
			<Fa icon={faWarning} />
			You have unsaved changes
		{/if}
	</span>
	<button
		class="button primary"
		disabled={status !== Status.Changed || saveDisabled}
		on:click={() => dispatch("save")}
	>
		<Fa icon={faSave} />
		Save
	</button>
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	@keyframes scale {
		0% {
			transform: scale(1);
		}
		50% {
			transform: scale(1.05);
		}
		100% {
			transform: scale(1);
		}
	}

	.bar {
		position: absolute;
		bottom: 1rem;
		left: 0;
		right: 0;
		margin-left: auto;
		margin-right: auto;
		max-width: 25rem;

		font-weight: 500;

		background-color: $bgColor2;
		padding: 1rem;
		border-radius: 0.5rem;
		border: $borderColor solid 1px;

		display: flex;
		align-items: center;
		justify-content: space-between;

		transform: translateY(calc(100% + 1rem));
		transition: transform 0.2s;

		&.shown {
			transform: translateY(0);
		}

		&.highlighted {
			border-color: $errorColor;
			animation: scale 0.25s;
		}
	}

	@media screen and (max-width: $mobileBreakpoint) {
		.bar {
			&.shown {
				// Space for bottom nav bar on mobile
				transform: translateY(-3rem);
			}
		}
	}

	.status {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.button {
		padding: 0.4rem 0.8rem;
		font-weight: 500;
		font-size: 0.9rem;

		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;

		&:disabled {
			opacity: 0.5;
		}
	}
</style>
