<script lang="ts">
	import { faChevronDown } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";
</script>

<button class="show-more" on:click>
	<h4>show more</h4>
	<Fa icon={faChevronDown} />
	<hr />
</button>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.show-more {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		color: $primaryColor;

		margin-top: 0.5rem;

		&:hover,
		&:focus-visible {
			color: $primaryColorLight;
		}

		& > h4 {
			margin: 0;
			font-size: 0.95rem;
			font-weight: 400;
		}

		& > hr {
			border-style: solid;
			flex-grow: 1;
			color: $bgColorLight;
		}
	}
</style>
