<script lang="ts">
	import { beforeNavigate, goto } from "$app/navigation";
	import { page } from "$app/stores";
	import LeakWarning from "./settings/leak-warning.svelte";

	type Tab = {
		name: string;
		pathname: string;
		showWarning?: boolean;
	};

	export let tabs: Tab[];

	let continueLink: string | undefined;

	beforeNavigate((nav) => {
		if (nav.to) {
			const tab = tabs.find((tab) => tab.pathname === nav.to?.url.pathname);
			if (tab?.showWarning && !continueLink) {
				continueLink = tab.pathname;
				nav.cancel();
			} else {
				continueLink = undefined;
			}
		}
	});

	function onContinue() {
		if (continueLink) {
			goto(continueLink);
		}
	}
</script>

<nav class="selector" aria-label="Tab selector">
	{#if continueLink}
		<LeakWarning on:close={() => (continueLink = undefined)} on:continue={onContinue} />
	{/if}
	{#each tabs as tab}
		<a class:selected={$page.url.pathname === tab.pathname} href={tab.pathname} draggable="false"
			>{tab.name}</a
		>
	{/each}
</nav>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.selector {
		display: flex;
		flex-wrap: wrap;
		column-gap: 1rem;
		row-gap: 0.25rem;

		user-select: none;

		& > a {
			padding: 0.4rem 1rem;
			border: none;
			border-radius: 0.25rem;
			background-color: none;
			font-weight: 500;
			color: $textColorLight;
			text-decoration: none;

			&.selected {
				background-color: $primaryColor;
				color: $textColor;
			}

			&:not(.selected):hover,
			&:not(.selected):focus-visible {
				background-color: rgba($primaryColor, 0.25);
			}
		}
	}
</style>
