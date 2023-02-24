<script lang="ts">
	import { page } from "$app/stores";

	const isNotFound = $page.status === 404;
	const title = isNotFound ? "Not Found" : "Internal Error";
</script>

<svelte:head>
	<title>Scuffle - {title}</title>
</svelte:head>

<div class="container">
	<div class="inner">
		<h1>{title}</h1>
		<p>
			{#if isNotFound}
				The page you were looking for could not be found.
			{:else}
				An internal error occurred.
				<br />
				You can try refreshing the page.
				<br />
				If the problem persists, please contact us.
			{/if}
		</p>
		<a href="/">Go back Home</a>
	</div>
</div>

<style lang="scss">
	.container {
		display: grid;
		place-items: center;
		height: 100vh;
	}

	.inner {
		text-align: center;
		font-size: 1.5rem;
	}
</style>
