<script lang="ts">
	import { loginMode } from "$store/login";
	import { sideNavOpen } from "$store/layout";
	import AlignLeft from "$icons/align-left.svelte";
	import LogoText from "$icons/logo-text.svelte";
	import Search from "$icons/search.svelte";
	import Login from "$icons/login.svelte";

	function openLogin() {
		loginMode.set(1);
	}

	function openSignup() {
		loginMode.set(2);
	}

	function toggleSideNav() {
		sideNavOpen.update((v) => !v);
	}

	function search(e: Event) {
		// When the query is not empty we can let the form submit
		if (!query) {
			e.preventDefault();
			queryInputRef.focus();
		}
	}

	let queryInputRef: HTMLInputElement;
	let query = "";
</script>

<nav class="main-grid">
	<div class="logo-container">
		<button class="toggle-side-nav" class:toggled={!$sideNavOpen} on:click={toggleSideNav}>
			<AlignLeft />
		</button>
		<a href="/" class="logo-link">
			<LogoText />
		</a>
	</div>
	<!-- This form even works with JS disabled -->
	<form class="search-container" on:submit={search} method="get" action="/search">
		<input name="q" type="text" placeholder="SEARCH" bind:this={queryInputRef} bind:value={query} />
		<button class="search-button">
			<Search />
		</button>
	</form>
	<div class="nav-right">
		<div class="buttons">
			<button class="login button" on:click={openLogin}>
				<span class="icon-login"><Login /></span><span>Log in</span>
			</button>
			<button class="signup button" on:click={openSignup}>
				<span>Sign up</span>
			</button>
		</div>
	</div>
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	nav {
		display: flex;
		justify-content: space-between;
		align-items: stretch;
		background-color: $bgColor2;
		height: $topNavHeight;
		z-index: 5;
		padding: 0.25rem 0.75rem;

		gap: 1rem;
	}

	.main-grid {
		position: sticky;
		top: 0;
		grid-area: top-nav;
	}

	.logo-container {
		display: flex;
		align-items: center;

		.toggle-side-nav {
			background-color: unset;
			font: inherit;
			color: inherit;
			cursor: pointer;
			border: 0;
			outline: 0;
			padding: 0;
			margin: 0;
			display: flex;
			align-items: center;
			&.toggled {
				transform: rotate(180deg);
			}
		}

		.logo-link {
			color: inherit;
			text-decoration: none;
			display: flex;
			align-items: center;
			font-size: 1.75rem;
		}
	}

	.search-container {
		flex-grow: 1;
		max-width: 30rem;

		display: flex;
		justify-content: center;
		align-items: stretch;

		input {
			flex-grow: 1;
			width: 6rem;
			border: 1px solid $borderColor;
			border-right: none;
			border-radius: 1rem 0 0 1rem;
			transition: border-color 0.25s;
			padding: 0.5rem 1rem;
			font: inherit;
			background-color: $bgColor2;
			color: white;
			font-weight: 500;
			outline: 0;
			&:focus {
				border-color: #ff7357;
				background-color: black;
			}
			&::placeholder {
				color: #ffffff70;
			}
		}

		.search-button {
			border: 1px solid $borderColor;
			border-radius: 0 1rem 1rem 0;
			border-left: none;
			transition: border-color 0.25s;

			padding: 0.4rem;
			font-size: 2.5rem;
			color: white;
			background-color: $bgColor2;
			cursor: pointer;
		}

		input:focus + .search-button {
			background-color: $bgColor;
			border-color: #ff7357;
		}
	}

	.buttons,
	.nav-right {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.button {
		color: white;
		border-radius: 0.6rem;
		transition:
			background-color 0.5s,
			color 0.5s,
			box-shadow 0.5s,
			border-color 0.5s;
		cursor: pointer;
		padding: 0.6rem 0.8rem;
		font: inherit;
		border: 1px solid #2c2c2c;
		background: #16181862;
		&.login {
			display: flex;
			align-items: center;
			justify-content: center;
			column-gap: 0.25rem;
			color: #a0a0a0;

			&:hover {
				color: white;
				border-color: white;
			}

			.icon-login {
				font-size: 1.5rem;
				display: flex;
				align-items: center;
			}
		}
		&.signup {
			background-color: white;
			color: black;
			border-radius: 0.8rem;
			&:hover {
				filter: drop-shadow(0 0 5px rgba(255, 255, 255, 0.5));
			}
		}
	}
</style>
