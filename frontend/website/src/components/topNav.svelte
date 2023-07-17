<script lang="ts">
	import { loginMode, sessionToken } from "$/store/login";
	import { user } from "$store/user";
	import { faSearch, faUser } from "@fortawesome/free-solid-svg-icons";
	import Fa from "svelte-fa";
	import MouseTrap from "./mouseTrap.svelte";
	import { focusTrap } from "$lib/focusTrap";
	import TransitionCloser from "./transitionCloser.svelte";

	let pfpDropdownOpen = false;

	function openLogin() {
		loginMode.set(1);
	}

	function openSignup() {
		loginMode.set(2);
	}

	let lastUpdate = 0;
	function setPfpDropdown(state?: boolean) {
		// This is a hack to prevent the dropdown from closing when the user clicks on the pfp button.
		if (Date.now() - lastUpdate < 100) return;
		lastUpdate = Date.now();
		pfpDropdownOpen = state ?? !pfpDropdownOpen;
	}

	function closePfpDropdown() {
		setPfpDropdown(false);
	}

	function logout() {
		// When we set this to null the store will delete the token from local storage & invalidate the token on the server.
		// Will also update the value of $user to null.
		sessionToken.set(null);
		closePfpDropdown();
	}
</script>

<nav class="main-grid">
	<div class="logo" />
	<div class="search">
		<input type="text" placeholder="Search" />
		<div class="icon">
			<Fa icon={faSearch} />
		</div>
	</div>
	<div class="nav-right">
		<!-- TODO
			We should figure out a way to make this work when the page is first loaded.
			Currently it flashes the login/signup buttons before the user is loaded.
			Perhaps we can have a loading state for the user store?
			if loading render a button that has no text and is very minimal.
		-->
		<div class="buttons" use:focusTrap={pfpDropdownOpen}>
			{#if $user}
				<button class="pfp" on:click={() => setPfpDropdown()}>
					<Fa icon={faUser} />
				</button>
				<TransitionCloser
					open={pfpDropdownOpen}
					closeAnimationDuration={150}
					openAnimationDuration={150}
					inheritAll={false}
				>
					<MouseTrap on:close={closePfpDropdown} inheritAll={false}>
						<div class="pfp-dropdown">
							<a class="pfp-button" href={`/${$user.username}`} on:click={closePfpDropdown}
								>Channel</a
							>
							<a class="pfp-button" href="/profile" on:click={closePfpDropdown}>Profile</a>
							<button class="pfp-button" on:click={logout}>Logout</button>
						</div>
					</MouseTrap>
				</TransitionCloser>
			{:else}
				<button class="login button" on:click={openLogin}>Login</button>
				<button class="signup button" on:click={openSignup}>Sign up</button>
			{/if}
		</div>
	</div>
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.main-grid {
		position: sticky;
		top: 0;
		grid-column: 2 / 2;
		grid-row: 1 / 1;
	}

	nav {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		justify-content: space-between;
		background-color: $bgColor2;
		height: $topNavHeight;
		width: 100%;
		z-index: 5;
	}

	.search {
		display: flex;
		width: 30rem;
		position: relative;

		& > input {
			border: 3px solid $borderColor;
			border-radius: 1rem;
			padding: 0.5rem 1rem;
			font: inherit;
			background-color: $bgColor2;
			color: white;
			width: 100%;
			padding-right: 2rem;
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
			right: 1rem;
			top: 0.7rem;
			color: #ffffff70;
		}
	}

	.buttons,
	.nav-right {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.nav-right {
		padding: 0.5rem;
		gap: 1rem;
		justify-content: flex-end;
	}

	.button {
		color: white;
		border-radius: 0.4rem;
		transition:
			background-color 0.5s,
			color 0.5s,
			box-shadow 0.5s;
		cursor: pointer;
		padding: 0.5rem 1rem;
		margin: 0 0.5rem;
		font: inherit;
		border: 1px solid #96491c;
		background: #16181862;
		&.signup {
			background-color: #cf634d;
			&:hover {
				background-color: #f79986;
				box-shadow: 0px 6px 20px 7px rgba(207, 69, 41, 0.2);
			}
		}

		&.login {
			&:hover {
				box-shadow: 0px 6px 20px 7px rgba(207, 69, 41, 0.1);
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
		transition:
			background-color 0.5s,
			color 0.5s,
			box-shadow 0.5s;
		cursor: pointer;
		border: 0.1rem solid #96491c;
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
		background-color: #161818;
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
			cursor: pointer;
			width: 100%;
			text-align: center;
			&:focus,
			&:hover {
				outline: none;
				background-color: #f79986;
			}
		}
	}
</style>
