<script context="module">
	let dropDownIndex = 0;
</script>

<script lang="ts">
	import { mouseTrap } from "$/lib/utils";

	let index = dropDownIndex;
	dropDownIndex += 1;

	let expanded = false;

	function toggle() {
		expanded = !expanded;
	}

	function close() {
		expanded = false;
	}
</script>

<button
	on:click={toggle}
	aria-expanded={expanded}
	aria-controls="dropdown-list-{index}"
	use:mouseTrap={close}
>
	<slot />
	{#if expanded}
		<ul class="list" id="dropdown-list-{index}">
			<slot name="dropdown" />
		</ul>
	{/if}
</button>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	button {
		position: relative;

		.list {
			display: flex;
			flex-direction: column;

			z-index: 1;

			position: absolute;
			right: 0;
			width: 10rem;
			margin: 0;
			padding: 0;
			border: $borderColor 1px solid;

			list-style-type: none;

			background-color: $bgColor;
			filter: drop-shadow(0 0 0.25rem rgba(0, 0, 0, 0.25));

			:global(li) {
				display: flex;
				flex-direction: column;
				align-items: stretch;

				&:hover,
				&:focus-visible {
					background-color: $bgColorLight;
				}
			}

			:global(a),
			:global(button) {
				padding: 0.5rem 0.75rem;

				text-align: left;
				color: $textColor;
				text-decoration: none;
				font-weight: 500;

				display: flex;
				align-items: center;
				gap: 0.5rem;
			}
		}
	}
</style>
