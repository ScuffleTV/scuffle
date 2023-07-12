<script context="module">
	let dropDownIndex = 0;
</script>

<script lang="ts">
	import MouseTrap from "./mouse-trap.svelte";

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

<MouseTrap on:close={close}>
	<button on:click={toggle} aria-expanded={expanded} aria-controls={`dropdown-list-${index}`}>
		<slot />
		{#if expanded}
			<ul class="list" id={`dropdown-list-${index}`}>
				<slot name="dropdown" />
			</ul>
		{/if}
	</button>
</MouseTrap>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	button {
		position: relative;

		.list {
			display: flex;
			flex-direction: column;

			position: absolute;
			right: 0;
			width: 10rem;

			background-color: $bgColor;
			filter: drop-shadow(0 0 0.25rem rgba(0, 0, 0, 0.25));

			:global(a),
			:global(button) {
				color: $textColor;
				text-decoration: none;
				padding: 0.5rem 0.75rem;
				font-weight: 500;
				text-align: left;

				&:hover,
				&:focus-visible {
					background-color: $bgColorLight;
				}
			}
		}
	}
</style>
