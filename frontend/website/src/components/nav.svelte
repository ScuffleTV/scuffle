<script lang="ts">
	import { loginMode, sessionToken } from "../store/login";
	import { page } from "$app/stores";
	import { user } from "../store/user";
	import { faUser } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";
	import MouseTrap from "./mouseTrap.svelte";
	import { focusTrap } from "$lib/focusTrap";
	import TransitionCloser from "./transitionCloser.svelte";
	import { onMount } from "svelte";

	interface MenuItem {
		text: string;
		url: string;
	}

	let menuItems: MenuItem[] = [
		{ text: "Scuffle", url: "/" },
		{ text: "About", url: "/about" },
	];

	let activeMenuItem: MenuItem | undefined;
	let pfpDropdownOpen = false;

	// Subscribe to the page store to update the active menu item when the page changes.
	// This is also called when the page first loads and on SSR.
	onMount(() =>
		page.subscribe((value) => {
			activeMenuItem = menuItems.find((item) => item.url === value.url.pathname);
			pfpDropdownOpen = false;
		}),
	);

	function openLogin() {
		loginMode.set(1);
	}

	function openSignup() {
		loginMode.set(2);
	}

	function setPfpDropdown(state?: boolean) {
		pfpDropdownOpen = state ?? !pfpDropdownOpen;
	}

	function logout() {
		// When we set this to null the store will delete the token from local storage & invalidate the token on the server.
		// Will also update the value of $user to null.
		sessionToken.set(null);
		pfpDropdownOpen = false;
	}
</script>

<nav>
	<div class="logo" />
	<div class="nav-right">
		<div class="menu">
			{#each menuItems as item}
				<a href={item.url} class="menu-item" class:active-page={activeMenuItem === item}>
					{item.text}
				</a>
			{/each}
		</div>
		<!-- TODO
			We should figure out a way to make this work when the page is first loaded.
			Currently it flashes the login/signup buttons before the user is loaded.
			Perhaps we can have a loading state for the user store?
			if loading render a button that has no text and is very minimal.
		-->
		{#if $user}
			<button class="pfp" on:click|stopPropagation={() => setPfpDropdown()}>
				<Fa icon={faUser} />
			</button>
			<TransitionCloser
				open={pfpDropdownOpen}
				closeAnimationDuration={150}
				openAnimationDuration={150}
				inheritAll={false}
			>
				<MouseTrap on:close={() => setPfpDropdown(false)} inheritAll={false}>
					<div class="pfp-dropdown">
						<a class="pfp-button" href="/profile">Profile</a>
						<button class="pfp-button" on:click={logout}>Logout</button>
					</div>
				</MouseTrap>
			</TransitionCloser>
		{:else}
			<div class="buttons" use:focusTrap={pfpDropdownOpen}>
				<button class="login button" on:click={openLogin}>Login</button>
				<button class="signup button" on:click={openSignup}>Sign up</button>
			</div>
		{/if}
	</div>
</nav>

<style lang="scss">
	a.active-page {
		text-decoration: underline;
		text-underline-offset: 0.5rem;
		text-decoration-color: #ff7357;
		text-decoration-thickness: 0.2rem;
	}

	nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background-color: #0000004b;
		backdrop-filter: blur(16px) saturate(125%);
	}

	.menu,
	.buttons,
	.nav-right {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.nav-right {
		padding: 0.5rem;
		grid-gap: 0.5rem;
		margin-right: 0.5rem;
	}

	.menu {
		display: flex;
		grid-gap: 0.5rem;
	}

	.menu-item {
		font-size: 1.1rem;
		color: white;
		padding: 1rem 0.5rem;
		&:hover {
			color: #ff7357;
		}
	}

	.button {
		color: white;
		border-radius: 0.4rem;
		transition: background-color 0.5s, color 0.5s, box-shadow 0.5s;
		cursor: pointer;
		padding: 0.5rem 1rem;
		margin: 0 0.5rem;
		font: inherit;
		border: 1px solid #96491c;
		background: #16181862;
		box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.1);
		&.signup {
			background-color: #cf634d;
			&:hover {
				background-color: #f79986;
				box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.2);
			}
		}

		&.login {
			&:hover {
				background-color: #ff7357;
			}
		}
	}

	.pfp {
		display: flex;
		place-items: center;
		justify-content: center;
		font-size: 1.5rem;
		border-radius: 50%;
		width: 3rem;
		height: 3rem;
		color: white;
		transition: background-color 0.5s, color 0.5s, box-shadow 0.5s;
		cursor: pointer;
		border: 0.1rem solid #96491c;
		box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.1);
		background-color: #212121;
		&:hover {
			background-color: #f79986;
			box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.2);
		}
	}

	.pfp-dropdown {
		position: absolute;
		top: 100%;
		right: 1rem;
		background-color: #16181862;
		border-radius: 0.2rem;
		box-shadow: 0px 6px 20px 7px rgba(255, 115, 87, 0.1);
		border: 1px solid #96491c;
		overflow: hidden;
		.pfp-button {
			background-color: transparent;
			margin: 0;
			border: 0;
			font: inherit;
			display: block;
			color: white;
			padding: 1rem 3rem;
			border-radius: 0.1rem;
			&:focus,
			&:hover {
				outline: none;
				background-color: #f79986;
			}
		}
	}
</style>
