<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";
	import MouseTrap from "../mouse-trap.svelte";
	import { fade } from "svelte/transition";
	import Fa from "svelte-fa";
	import { faExclamationTriangle } from "@fortawesome/free-solid-svg-icons";

	const dispatch = createEventDispatcher();

	let dialog: HTMLDialogElement;

	onMount(() => {
		dialog.showModal();
	});

	function close() {
		dispatch("close");
	}
</script>

<dialog bind:this={dialog} aria-modal="true" transition:fade={{ duration: 100 }}>
	<MouseTrap on:close={close}>
		<div class="title-container">
			<h1 class="heading">
				<Fa icon={faExclamationTriangle} />
				<span>Leak Warning</span>
			</h1>
			<span class="live">Live</span>
		</div>
		<p class="text">You are live, are you sure you want to reveal your personal data?</p>
		<div class="buttons">
			<button class="button primary" on:click={close}>Go Back</button>
			<button class="button secondary" on:click={() => dispatch("continue")}>Continue</button>
		</div>
	</MouseTrap>
</dialog>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	dialog {
		background: linear-gradient(to bottom, #18191a, #101415);
		color: $textColor;
		box-shadow: 0 0 0.5rem 0.5rem rgba(0, 0, 0, 0.3);

		width: min(30rem, 90vw);
		padding: 2.5rem;
		border: none;
		border-radius: 0.25rem;
		font-weight: 500;

		&::backdrop {
			background-color: rgba(0, 0, 0, 0.5);
		}
	}

	.title-container {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.heading {
		font-size: 1.8rem;

		display: flex;
		align-items: center;
		gap: 0.5rem;

		& > span {
			font-size: 2rem;
		}
	}

	.live {
		font-weight: 500;
		color: $textColor;

		&::before {
			content: "";
			display: inline-block;
			width: 0.4rem;
			height: 0.4rem;
			background-color: $liveColor;
			border-radius: 50%;
			margin-right: 0.4rem;
			margin-bottom: 0.1rem;
		}
	}

	.text {
		font-weight: 500;
		color: $textColorLight;
	}

	.buttons {
		display: flex;
		align-items: center;
		gap: 1rem;
		justify-content: flex-end;

		& > .button {
			padding: 0.4rem 0.8rem;
		}
	}
</style>
