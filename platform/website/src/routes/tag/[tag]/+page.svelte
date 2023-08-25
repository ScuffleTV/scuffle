<script lang="ts">
	import type { PageData } from "./$types";

	export let data: PageData;
</script>

<div class="content">
	<span>Tag</span>
	<h1>{data.tag}</h1>
</div>

<style lang="scss">
	.content {
		grid-area: content;
	}
</style>
