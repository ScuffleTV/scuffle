<script lang="ts">
	import { AuthMode, authDialog, sessionToken, user } from "$/store/auth";
	import { sideNavCollapsed, topNavHidden } from "$store/layout";
	import LogoText from "./icons/logo-text.svelte";
	import Fa from "svelte-fa";
	import {
		faChevronLeft,
		faArrowRightToBracket,
		faUser,
		faCog,
		faArrowRightFromBracket,
	} from "@fortawesome/free-solid-svg-icons";
	import DropDown from "./drop-down.svelte";
	import { logout } from "$/lib/auth";
	import DefaultAvatar from "./user/default-avatar.svelte";
	import { getContextClient } from "@urql/svelte";
	import Search from "./top-nav/search.svelte";
	import Logo from "./icons/logo.svelte";
	import ResponsiveContainer from "../components/responsive-container.svelte";
	import LiveIndicator from "./top-nav/live-indicator.svelte";

	const client = getContextClient();

	function openLogin() {
		$authDialog = {
			opened: true,
			mode: AuthMode.Login,
		};
	}

	function openSignup() {
		$authDialog = {
			opened: true,
			mode: AuthMode.Register,
		};
	}

	function onLogoutClick() {
		logout(client, $sessionToken).then(() => ($sessionToken = null));
	}

	function toggleSideNav() {
		sideNavCollapsed.update((v) => !v);
	}
</script>

<nav class:hidden={$topNavHidden} aria-label="Top navigation">
	<div class="logo-container">
		<button
			class="toggle-side-nav hide-on-mobile"
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
			<ResponsiveContainer>
				<LogoText slot="desktop" />
				<Logo height={1.75 * 16} slot="mobile" />
			</ResponsiveContainer>
		</a>
	</div>
	<Search />
	<div class="nav-right">
		{#if $user}
			<LiveIndicator />
			<DropDown>
				<DefaultAvatar userId={$user.id} displayColor={$user.displayColor} />
				<svelte:fragment slot="dropdown">
					<li>
						<a href="/{$user.username}">
							<Fa icon={faUser} />
							Profile
						</a>
					</li>
					<li>
						<a href="/settings">
							<Fa icon={faCog} />
							Settings
						</a>
					</li>
					<li>
						<button on:click={onLogoutClick}>
							<Fa icon={faArrowRightFromBracket} />
							Log out
						</button>
					</li>
				</svelte:fragment>
			</DropDown>
		{:else}
			<div class="buttons">
				<button class="login button secondary" on:click={openLogin}>
					<span class="icon-login">
						<Fa icon={faArrowRightToBracket} size="1.2x" />
					</span>
					<span>Log in</span>
				</button>
				<button class="signup button primary hide-on-mobile" on:click={openSignup}>
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

	.logo-container {
		/* Take all available space but shrink by a very high factor */
		flex: 1 9999;

		display: flex;
		align-items: center;
		gap: 0.5rem;

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

	.nav-right {
		/* Take all available space but shrink by a very high factor */
		flex: 1 9999;
	}

	.buttons,
	.nav-right {
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
