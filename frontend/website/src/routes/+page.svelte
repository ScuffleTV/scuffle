<script lang="ts">
</script>

<svelte:head>
	<title>Scuffle - Home</title>
</svelte:head>

<div class="main">
	<div class="discover">
		<h1>Discover</h1>
		<span>Discover something new</span>
	</div>

	<div class="content">
		<h1><i>batman's chest</i></h1>
	</div>
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.main {
		display: grid;
		grid-template-rows: auto 1fr auto;
		grid-template-columns: auto 1fr auto;
		min-height: 100%;
		grid-column: 2 / 2;
		grid-row: 2 / 2;
	}

	.discover {
		padding: 1rem 3rem;
		border-bottom: 0.1rem solid $borderColor;
		position: sticky;
		top: 0;
		width: 100%;
		background-color: $bgColor2;
		grid-row: 1 / 1;
		grid-column: 1 / -1;
		top: $topNavHeight;

		h1 {
			font-weight: 600;
			font-size: 2.5rem;
			color: #ff7357;
		}

		span {
			font-weight: 400;
			font-size: 1.25rem;
			color: #6d6d6d;
		}
	}

	.content {
		background-color: $bgColor2;
		padding: 1rem;
		height: 300vh;
		grid-row: 2 / -1;
		grid-column: 1 / -1;
	}
</style>
