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
	<div class="search-container">
		<div class="search">
			<input type="text" placeholder="SEARCH" />
			<div class="icon">
				<Search />
			</div>
		</div>
	</div>
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

	.main-grid {
		position: sticky;
		top: 0;
		grid-area: top-nav;
	}

	.logo-container {
		display: flex;
		align-items: center;
		padding: 0 0.5rem;

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

	nav {
		display: grid;
		grid-template-columns: auto 1fr auto;
		align-items: center;
		justify-content: space-between;
		background-color: $bgColor2;
		height: $topNavHeight;
		width: 100%;
		z-index: 5;
		padding: 0.25rem;
		> * {
			height: 100%;
		}
	}

	.search-container {
		display: grid;
		place-items: center;

		.search {
			position: relative;
			width: 30rem;
			height: 100%;

			& > input {
				border: 1px solid $borderColor;
				border-radius: 1rem;
				padding: 0.5rem 1rem;
				font: inherit;
				background-color: $bgColor2;
				color: white;
				width: 100%;
				height: 100%;
				padding-right: 2rem;
				font-weight: 500;
				outline: 0;
				transition: border-color 0.25s;
				&:focus {
					border-color: #ff7357;
					background-color: black;
				}
				&::placeholder {
					color: #ffffff70;
				}
			}

			& > .icon {
				position: absolute;
				right: 0.25rem;
				top: 0.4rem;
				font-size: 2.5rem;
				color: white;
			}
		}
	}

	.buttons,
	.nav-right {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
	}

	.nav-right {
		padding: 0.25rem 0.75rem;
		gap: 1rem;
		justify-content: flex-end;
	}

	.button {
		color: white;
		border-radius: 0.6rem;
		transition: background-color 0.5s, color 0.5s, box-shadow 0.5s, border-color 0.5s;
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
