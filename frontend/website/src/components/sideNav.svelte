<script lang="ts">
	import CloseSidebar from "./closeSidebar.svelte";
	import ScuffleLogo from "./scuffleLogo.svelte";

	export let collapsed = false;

	function collapseNav() {
		collapsed = !collapsed;
	}
</script>

<nav class:collapsed class="main-grid">
	<div class="top">
		<a href="/" class="logo">
			<ScuffleLogo />
		</a>
		<button class="collapse-icon" on:click={collapseNav}>
			<CloseSidebar />
		</button>
	</div>
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.main-grid {
		grid-row: 1 / -1;
		grid-column: 1 / 1;
	}

	.top {
		display: grid;
		padding: 1rem;
		grid-template-columns: 1fr auto;
	}

	nav {
		position: sticky;
		top: 0;
		height: 100vh;
		z-index: 2;
		width: $sideNavWidth;
		background-color: $bgColor2;
		display: grid;
		grid-template-rows: auto 1fr;

		border-right: 0.1rem solid $borderColor;

		.top {
			.logo {
				grid-column: 1 / 1;
			}
			.collapse-icon {
				transform: rotate(0);
				grid-column: 2 / 2;
			}
		}

		&.collapsed {
			width: 5rem;
			.top {
				grid-template-columns: 1fr;
				grid-template-rows: auto auto;
				.logo {
					grid-column: 1 / 1;
					grid-row: 1 / 1;
				}
				.collapse-icon {
					transform: rotate(180deg);
					grid-column: 1 / 1;
					grid-row: 2 / 2;
				}
			}
		}
	}

	.collapse-icon {
		border: 0;
		outline: 0;
		background-color: transparent;
		padding: 0.5rem;
		cursor: pointer;
		transition: color 0.25s ease;
		color: #ffffff70;
		font-size: 2rem;
		width: 3rem;
		height: 3rem;
		grid-row: 1 / 1;
		&:hover {
			color: #f79986;
		}
	}

	.logo {
		border: 0;
		outline: 0;
		background-color: #2f2f2f;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		padding: 0.25rem;
		font-size: 2rem;
		width: 3rem;
		height: 3rem;
		transition:
			background-color 0.25s ease,
			box-shadow 0.25s ease;
		grid-row: 1 / 1;
		&:hover {
			background-color: #f79986;
			box-shadow: 0px 6px 20px 7px rgba(207, 69, 41, 0.2);
		}
	}
</style>
