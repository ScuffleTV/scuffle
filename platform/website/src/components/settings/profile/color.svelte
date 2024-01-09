<script lang="ts">
	import { createEventDispatcher } from "svelte";

	export let rgb: string;

	const dispatch = createEventDispatcher();
</script>

<button class="color" style="background-color: {rgb}" on:click={() => dispatch("click")} />

<style lang="scss">
	@import "../../../assets/styles/variables.scss";

	.color {
		width: 2rem;
		height: 2rem;
		border-radius: 50%;
	}
</style>
