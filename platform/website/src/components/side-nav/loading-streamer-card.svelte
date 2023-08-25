<script lang="ts">
	export let collapsed = false;
</script>

<div class="card">
	<div class="avatar"></div>
	{#if !collapsed}
		<div class="name"></div>
	{/if}
</div>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.card {
		padding: 0.5rem 0.75rem;
		padding-left: 0.625rem;

		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.avatar {
		width: 2rem;
		height: 2rem;
		background-color: $bgColorLight;
		border-radius: 50%;
	}

	.name {
		height: 0.5rem;
		width: 8rem;
		background-color: $bgColorLight;
		border-radius: 0.25rem;
	}
</style>
