<script lang="ts">
	import { loginMode } from "../store/login";
	import { page } from "$app/stores";

	interface MenuItem {
		text: string;
		url: string;
	}

	let menuItems: MenuItem[] = [
		{ text: "Scuffle", url: "/" },
		{ text: "About", url: "/about" },
	];

	let activeMenuItem: MenuItem | undefined;

	// Subscribe to the page store to update the active menu item when the page changes.
	// This is also called when the page first loads and on SSR.
	page.subscribe((value) => {
		activeMenuItem = menuItems.find((item) => item.url === value.url.pathname);
	});

	function openLogin() {
		loginMode.set(1);
	}

	function openSignup() {
		loginMode.set(2);
	}
</script>

<nav>
	<div class="logo" />
	<div class="nav-right">
		<div class="menu">
			{#each menuItems as item}
				<a href={item.url} class:active-page={activeMenuItem === item}>
					{item.text}
				</a>
			{/each}
		</div>
		<div class="buttons">
			<button class="login button" on:click={openLogin}>Login</button>
			<button class="signup button" on:click={openSignup}>Sign up</button>
		</div>
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
		padding: 1rem;
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

	.menu {
		margin-right: 2.5rem;
		a {
			margin: 0 1rem;
			font-size: 1.1rem;
			color: white;
			&:hover {
				color: #ff7357;
			}
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
</style>
