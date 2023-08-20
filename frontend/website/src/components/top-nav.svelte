<script lang="ts">
	import { loginMode, sessionToken } from "$store/login";
	import { sideNavCollapsed, topNavHidden } from "$store/layout";
	import { user } from "$store/user";
	import LogoText from "./icons/logo-text.svelte";
	import Avatar from "$/components/icons/avatar.svelte";
	import Fa from "svelte-fa";
	import {
		faChevronLeft,
		faArrowRightToBracket,
		faMagnifyingGlass,
	} from "@fortawesome/free-solid-svg-icons";
	import DropDown from "./drop-down.svelte";
	import { logout } from "$/lib/user";

	function openLogin() {
		$loginMode = 1;
	}

	function openSignup() {
		$loginMode = 2;
	}

	function onLogoutClick() {
		logout();
		$sessionToken = null;
		$user = null;
	}

	function toggleSideNav() {
		sideNavCollapsed.update((v) => !v);
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

<nav class="top-nav" class:hidden={$topNavHidden} aria-label="Top navigation">
	<div class="logo-container">
		<button
			class="toggle-side-nav"
			class:toggled={$sideNavCollapsed}
			on:click={toggleSideNav}
			aria-controls="side-nav"
			aria-expanded={!$sideNavCollapsed}
		>
			<span class="sr-only">Toggle sidebar</span>
			<Fa icon={faChevronLeft} fw size="1.2x" />
		</button>
		<a href="/" class="logo-link">
			<span class="sr-only">Home</span>
			<LogoText />
		</a>
	</div>
	<!-- This form even works with JS disabled -->
	<search>
		<form on:submit={search} method="get" action="/search">
			<input
				name="q"
				type="text"
				placeholder="SEARCH"
				bind:this={queryInputRef}
				bind:value={query}
			/>
			<button class="search-button" type="submit">
				<span class="sr-only">Search</span>
				<Fa icon={faMagnifyingGlass} size="1.2x" />
			</button>
		</form>
	</search>
	<div class="nav-right">
		{#if $user}
			<DropDown>
				<Avatar />
				<li slot="dropdown">
					<a href="/{$user.username}">Profile</a>
					<button on:click={onLogoutClick}>Log out</button>
				</li>
			</DropDown>
		{:else}
			<div class="buttons">
				<button class="login button secondary" on:click={openLogin}>
					<span class="icon-login">
						<Fa icon={faArrowRightToBracket} size="1.2x" />
					</span>
					<span>Log in</span>
				</button>
				<button class="signup button primary" on:click={openSignup}>
					<span>Sign up</span>
				</button>
			</div>
		{/if}
	</div>
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	nav {
		display: flex;
		justify-content: space-between;
		align-items: center;
		background-color: $bgColor2;
		height: $topNavHeight;
		padding: 0.25rem 0.75rem;

		gap: 1rem;

		border-bottom: 0.1rem solid $borderColor;

		&.hidden {
			display: none;
		}
	}

	.top-nav {
		grid-area: top-nav;
	}

	.logo-container {
		/* Take all available space but shrink by a very high factor */
		flex: 1 9999;

		display: flex;
		align-items: center;

		.toggle-side-nav {
			background-color: unset;
			font: inherit;
			color: $textColorLight;
			transition: color 0.25s;

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

			&:hover,
			&:focus-visible {
				color: $textColor;
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

	search {
		/* First, take 20rem and then shrink by a factor of 1 */
		flex: 0 1 20rem;

		& > form {
			/* First, take 20rem and then shrink by a factor of 1 */
			flex: 0 1 20rem;

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
				color: $textColor;
				font-weight: 500;
				outline: 0;
				&:focus {
					border-color: $primaryColor;
					background-color: black;
				}
				&::placeholder {
					color: $textColorLight;
				}
			}

			.search-button {
				border: 1px solid $borderColor;
				border-radius: 0 1rem 1rem 0;
				border-left: none;
				transition:
					border-color 0.25s,
					background-color 0.25s;

				padding: 0.75rem;
				color: $textColor;
				background-color: $bgColor2;
				cursor: pointer;

				display: flex;
				align-items: center;
			}

			input:focus + .search-button {
				background-color: $bgColor;
				border-color: $primaryColor;
			}
		}
	}

	.buttons,
	.nav-right {
		/* Take all available space but shrink by a very high factor */
		flex: 1 9999;

		display: flex;
		align-items: center;
		gap: 1rem;
		justify-content: flex-end;
	}

	.button {
		padding: 0.5rem 0.8rem;

		&.login {
			display: flex;
			align-items: center;
			justify-content: center;
			column-gap: 0.5rem;
		}
	}
</style>
