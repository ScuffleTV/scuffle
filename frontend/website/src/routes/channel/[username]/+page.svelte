<script lang="ts">
	// THIS SITE IS ONLY FOR TESTING CHAT
	import ChatRoom from "$components/chat/ChatRoom.svelte";
	import type { LoadParams } from "./+page";
	export let data: LoadParams;
</script>

<div class="channel">
	<ChatRoom chatId={data.chatId} />
</div>
