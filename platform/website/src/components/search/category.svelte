<script lang="ts">
	import type { Category } from "$/gql/graphql";
	import { createEventDispatcher } from "svelte";

	const dispatch = createEventDispatcher();

	export let category: Category;
</script>

<a on:click={() => dispatch("close")} href="/categories/{category.name.toLowerCase()}">
	<div class="avatar">
		<img src="/categories/minecraft.png" alt="Category banner" />
	</div>
	<div class="text-container">
		<span class="name ellipsis">{category.name}</span>
	</div>
</a>

<style lang="scss">
	@import "../../assets/styles/search-result.scss";

	.avatar > img {
		height: 100%;
	}
</style>
