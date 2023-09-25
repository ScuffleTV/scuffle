<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import Fa from "svelte-fa";
	import { faExclamationTriangle } from "@fortawesome/free-solid-svg-icons";
	import Dialog from "../dialog.svelte";

	const dispatch = createEventDispatcher();
</script>

<Dialog on:close>
	<div class="title-container">
		<h1 class="heading">
			<Fa icon={faExclamationTriangle} />
			<span>Leak Warning</span>
		</h1>
		<span class="live">Live</span>
	</div>
	<p class="text">You are live, are you sure you want to reveal your personal data?</p>
	<div class="buttons">
		<button class="button primary" on:click={() => dispatch("close")}>Go Back</button>
		<button class="button secondary" on:click={() => dispatch("continue")}>Continue</button>
	</div>
</Dialog>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

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
