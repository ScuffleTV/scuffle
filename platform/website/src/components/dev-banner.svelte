<script lang="ts">
	import Fa from "svelte-fa";
	import {
		faBug,
		faCalendar,
		faCircleInfo,
		faCode,
		faCodeBranch,
		faCodeCommit,
		faCube,
		faGears,
		faLink,
	} from "@fortawesome/free-solid-svg-icons";
	import Dialog from "./dialog.svelte";
	import {
		PUBLIC_CF_TURNSTILE_KEY,
		PUBLIC_GQL_ENDPOINT,
		PUBLIC_GQL_WS_ENDPOINT,
		PUBLIC_GQL_VERSION,
		PUBLIC_BASE_URL,
		PUBLIC_TWITTER_HANDLE,
	} from "$env/static/public";
	import { websocketOpen } from "$/store/websocket";
	import { dev } from "$app/environment";

	let showDebugDialog = false;

	function onKeyDown(event: KeyboardEvent) {
		if (event.ctrlKey && event.altKey && event.shiftKey && event.key === "D") {
			showDebugDialog = !showDebugDialog;
			event.preventDefault();
		}
	}
</script>

<svelte:window on:keydown={onKeyDown} />

{#if dev}
	<span class="dev-banner">
		<span>
			<Fa icon={faCode} />
			In Development
		</span>
		<span>
			<Fa icon={faCircleInfo} />
			Attention! This is a development server! Go to <a href="https://scuffle.tv">scuffle.tv</a>
		</span>
		<button class="button" on:click={() => (showDebugDialog = true)}>
			<Fa icon={faBug} />
			Show Debug Info
		</button>
	</span>
{/if}

{#if showDebugDialog}
	<Dialog on:close={() => (showDebugDialog = false)} width={40}>
		<div class="debug-container">
			<div class="title">
				<h1>Debug Info</h1>
				<code>Ctrl+Alt+Shift+D</code>
			</div>
			<span>
				<Fa icon={faCodeCommit} fw />
				Commit Hash:
				<code>{import.meta.env.VITE_GIT_COMMIT}</code>
			</span>
			<span>
				<Fa icon={faCalendar} fw />
				Commit Date:
				<code>{import.meta.env.VITE_GIT_COMMIT_DATE}</code>
			</span>
			<span>
				<Fa icon={faCodeBranch} fw />
				Branch:
				<code>{import.meta.env.VITE_GIT_BRANCH}</code>
			</span>
			<span>
				<Fa icon={faGears} fw />
				Build Date:
				<code>{import.meta.env.VITE_BUILD_DATE}</code>
			</span>
			<span>
				<Fa icon={faLink} fw />
				{#if websocketOpen}
					Websocket Connected
				{:else}
					Websocket Disconnected
				{/if}
			</span>
			<div>
				<span>
					<Fa icon={faCube} fw />
					Vite
				</span>
				<ul>
					{#each Object.entries(import.meta.env) as [key, value]}
						<li><code>{key}</code>: <code>{value}</code></li>
					{/each}
				</ul>
			</div>
			<div>
				<span>
					<Fa icon={faCube} fw />
					Environment Variables
				</span>
				<ul>
					{#each Object.entries( { PUBLIC_CF_TURNSTILE_KEY, PUBLIC_GQL_ENDPOINT, PUBLIC_GQL_WS_ENDPOINT, PUBLIC_GQL_VERSION, PUBLIC_BASE_URL, PUBLIC_TWITTER_HANDLE }, ) as [key, value]}
						<li><code>{key}</code>: <code>{value}</code></li>
					{/each}
				</ul>
			</div>
		</div>
	</Dialog>
{/if}

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.dev-banner {
		color: $textColor;
		background-color: $primaryColor;
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 0.25rem 0.5rem;
		width: 100vw;
		height: 2rem;

		span {
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}

		a {
			color: $textColor;
		}
	}

	.button {
		color: $textColor;
		border: 1px solid $textColor;
	}

	.debug-container {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		// font-weight: 400;
	}

	code {
		font-size: 0.9rem;
	}

	.title {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;

		code {
			background-color: black;
			border-radius: 0.25rem;
			padding: 0.25rem 0.5rem;
		}
	}

	ul {
		padding-left: 1rem;
		margin: 0;
		font-weight: 400;
	}
</style>
