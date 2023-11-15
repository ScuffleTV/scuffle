<script lang="ts">
	import type { LayoutData } from "./$types";

	export let data: LayoutData;
</script>

<svelte:head>
	<title>Scuffle - {data.user.displayName}</title>
	<meta name="description" content="Watch {data.user.displayName} live on Scuffle" />
	<meta
		name="keywords"
		content="scuffle, live, stream, watch, {data.user.displayName}, {data.user.username}"
	/>

	<!-- Open Graph -->
	<meta property="og:title" content="Scuffle - {data.user.displayName}" />
	<meta property="og:description" content="Watch {data.user.displayName} live on Scuffle" />
	<meta
		property="og:image"
		content="https://static-cdn.jtvnw.net/jtv_user_pictures/3773bfdd-110b-4911-b914-6f04362a1331-profile_image-70x70.png"
	/>
	<meta property="og:image:alt" content="{data.user.displayName}'s profile picture" />
</svelte:head>

<slot />
