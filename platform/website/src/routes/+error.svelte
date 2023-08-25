<script lang="ts">
	import { page } from "$app/stores";

	let title = $page.error?.message ?? "Error";
</script>

<svelte:head>
	<title>Scuffle - {title}</title>
</svelte:head>

<div class="container">
	<img src="/Sadge.webp" alt="Sadge" />
	<h1>{title}</h1>
	<span>
		{#if $page.status === 401}
			You are unauthorized to view this page.
		{:else if $page.status === 403}
			You do not have permission to view this page.
		{:else if $page.status === 404}
			The page you were looking for could not be found.
		{:else}
			An internal error occurred.
			<br />
			You can try refreshing the page.
			<br />
			If the problem persists, please contact us.
		{/if}
	</span>
	<a href="/">Go Back Home</a>
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.container {
		grid-area: content;

		height: 100%;

		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		text-align: center;

		font-size: 1.5rem;
	}

	img {
		filter: saturate(0) opacity(0.5);
	}

	a {
		color: $primaryColor;
		text-decoration: none;

		&:hover,
		&:focus-visible {
			color: $primaryColorLight;
			text-decoration: underline;
		}
	}
</style>
