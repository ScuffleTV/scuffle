<script lang="ts">
	import init, { add } from "player";
	import { onMount } from "svelte";

	onMount(async () => {
		await init();
		console.log(add(1, 2));
	});
</script>

<h1>Welcome to SvelteKit</h1>
<p>Visit <a href="https://kit.svelte.dev">kit.svelte.dev</a> to read the documentation</p>
