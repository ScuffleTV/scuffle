<script lang="ts">
	export let username: string, message: string, messageType: string;
</script>

<div class="chat-message">
	{#if messageType === "message"}
		<p class="content"><span class="username">{username}</span>: {message}</p>
	{/if}
	{#if messageType === "info"}
		<p class="content info">{message}</p>
	{/if}
</div>

<style lang="scss">
	.chat-message {
		font-size: 0.8rem;
		display: flex;
		align-items: flex-start;
		p {
			margin: 4px 0;
			overflow-wrap: break-word;
		}
		.username {
			font-weight: bold;
		}
		.content {
			font-weight: normal;
			overflow-wrap: anywhere;
			font-weight: 400;
			&.info {
				color: rgb(163, 163, 163);
			}
		}
	}
</style>
