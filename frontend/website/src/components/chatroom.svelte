<script lang="ts">
	import { pipe, subscribe, type Subscription } from "wonka";
	import { client } from "$lib/gql";
	import { graphql } from "$gql";
	import AlignLeft from "$icons/align-left.svelte";
	import { user } from "$store/user";
	import { onMount } from "svelte";
	import { MessageType, type ChatMessage, type User } from "$gql/graphql";

	export let collapsed = false;
	export let channelId: string;
	let svelteId = 1;

	function collapseNav() {
		collapsed = !collapsed;
		if (collapsed) {
			unsubscribeFromMessages();
		} else {
			subscribeToMessages();
		}
	}

	function createSystemMessage(content: string): ChatMessage {
		return {
			authorId: "",
			// We cast to User because this is a system message and we don't care about the author or the channel
			channel: {} as User,
			author: {} as User,
			channelId: channelId,
			createdAt: "",
			id: "",
			type: MessageType.System,
			content: content,
		};
	}

	interface MessageContainer {
		// Svelte uses this to track changes
		// Without it, svelte uses the index of the array as the key which causes issues when we remove items
		svelteId: number;
		message: ChatMessage;
	}

	const newMessageQuery = graphql(`
		mutation SendMessage($channelId: UUID!, $content: String!) {
			chat {
				sendMessage(channelId: $channelId, content: $content) {
					id
				}
			}
		}
	`);

	let chatStatus = "";
	let messages: MessageContainer[] = [];
	let subscription: Subscription;
	let chatMessage = "";
	let messagesEl: HTMLDivElement;
	let atBottom = true;

	$: valid = chatMessage.length > 0 && $user && chatStatus === "connected";

	function onScroll() {
		atBottom = messagesEl.scrollTop + messagesEl.offsetHeight >= messagesEl.scrollHeight - 30;
	}

	onMount(() => {
		subscribeToMessages();
		// Scroll to bottom
		messagesEl.addEventListener("scroll", onScroll);
		return () => {
			unsubscribeFromMessages();
			messagesEl.removeEventListener("scroll", onScroll);
		};
	});

	function scrollToBottom() {
		// We set at bottom to true so that if new messages are added we scroll to the bottom again
		atBottom = true;
		// We request an animation frame to make sure the scroll happens after the DOM has been updated
		window.requestAnimationFrame(() => {
			messagesEl.scrollTop = messagesEl.scrollHeight;
		});
	}

	function subscribeToMessages() {
		chatStatus = "connecting";
		const subscriptionQuery = graphql(`
			subscription ChatMessages($channelId: UUID!) {
				chatMessages(channelId: $channelId) {
					id
					content
					author {
						id
						username
						displayName
					}
				}
			}
		`);

		subscription = pipe(
			client.subscription(subscriptionQuery, { channelId: channelId }),
			subscribe((response) => {
				const message = response.data?.chatMessages as ChatMessage | undefined;
				if (message) {
					if (message.type === MessageType.Welcome) {
						chatStatus = "connected";
					}
					insertNewMessage(message);
				} else if (response.error) {
					insertNewMessage(createSystemMessage("Failed to connect to chat room"));
				}
			}),
		);
	}

	function insertNewMessage(message: ChatMessage) {
		const newMessage: MessageContainer = {
			svelteId: svelteId,
			message,
		};
		svelteId++;

		// Show 500 latest messages when scroll is at the bottom
		if (atBottom) {
			messages = [...messages, newMessage];
			if (messages.length > 500) {
				messages.shift();
			}
			scrollToBottom();
		} else {
			messages = [...messages, newMessage];
		}
	}

	function unsubscribeFromMessages() {
		chatStatus = "disconnected";
		subscription.unsubscribe();
		insertNewMessage(createSystemMessage("Disconnected from chat"));
	}

	async function sendMessageInner(message: string) {
		const response = await client
			.mutation(newMessageQuery, { channelId: channelId, content: message })
			.toPromise();
		if (response.error) {
			insertNewMessage(createSystemMessage("Failed to send message"));
		}
	}

	function sendMessage() {
		if (valid) {
			sendMessageInner(chatMessage);
			chatMessage = "";
		}
	}
</script>

{#if collapsed}
	<div class="uncollapse">
		<button class="collapse-icon" on:click={collapseNav}>
			<AlignLeft />
		</button>
	</div>
{/if}

<div class="chatroom" class:collapsed>
	<div class="top">
		<button class="collapse-icon" on:click={collapseNav}>
			<AlignLeft />
		</button>
		<span class="chat-title">Chat</span>
	</div>
	<div class="messages" bind:this={messagesEl}>
		{#each messages as message (message.svelteId)}
			<div class="message">
				{#if message.message.type === MessageType.User}
					<span class="message-sender">{message.message.author?.displayName}</span>:
					<span class="message-text">{message.message.content}</span>
				{/if}
				{#if message.message.type === MessageType.Welcome || message.message.type === MessageType.System}
					<span class="message-text info">{message.message.content}</span>
				{/if}
			</div>
		{:else}
			<div class="no-messages">Quiet in here...</div>
		{/each}
	</div>
	{#if atBottom === false}
		<div class="bottom-scroller-container">
			<button class="bottom-scroller" on:click={scrollToBottom}> Scroll to bottom </button>
		</div>
	{/if}
	<form class="chatbox" on:submit|preventDefault={sendMessage}>
		<input
			class="chatbox-input"
			type="text"
			maxlength="500"
			placeholder="Type a message..."
			bind:value={chatMessage}
			required
		/>
		<input class="chatbox-send" type="submit" value="Send" disabled={!valid} />
	</form>
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	$translucentColor: #ffffff70;

	.uncollapse {
		position: absolute;
		top: 0;
		right: 0;
		z-index: 6;
		transform: rotate(0deg) translateY(5rem) translateX(-1rem);
	}

	.chatroom {
		display: grid;
		position: sticky;
		top: 0;
		height: 100vh;
		grid-row: 1 / -1;
		grid-column: 3 / 3;
		background-color: $bgColor2;
		width: $chatroomWidth;
		grid-template-rows: auto 1fr auto;
		border-left: 0.1rem solid $borderColor;
		.collapse-icon {
			transform: rotate(180deg);
		}
		&.collapsed {
			display: none;
		}
	}

	.chat-title {
		font-size: 1.25rem;
		font-weight: 500;
		color: $textColor;
	}

	.top {
		padding: 1rem;
		display: flex;
		place-items: center;
		gap: 0.5rem;
		grid-row: 1 / 1;
	}

	.collapse-icon {
		display: flex;
		place-items: center;
		border: 0;
		outline: 0;
		background-color: transparent;
		padding: 0;
		cursor: pointer;
		transition: color 0.25s ease;
		color: $translucentColor;
		font-size: 1.75rem;
		grid-row: 1 / 1;
		&:hover {
			color: $primaryColorLight;
		}
	}

	.messages {
		grid-row: 2 / 2;
		overflow-y: scroll;
		overflow-x: hidden;
		display: flex;
		flex-direction: column;
	}

	.message {
		padding: 0.2rem 1rem;
		gap: 0.5rem;
		word-break: break-all;
		&-text {
			&.info {
				color: $translucentColor;
			}
		}
	}

	.chatbox {
		grid-row: 3 / 3;
		grid-column: 1 / -1;
		background-color: $bgColor2;
		padding: 0.5rem;
		display: grid;
		grid-template-rows: auto auto;
		grid-template-columns: 1fr auto;
		gap: 0.5rem;
	}

	.chatbox-input {
		border: 3px solid $borderColor;
		border-radius: 0.25rem;
		padding: 0.5rem 1rem;
		font: inherit;
		color: $textColor;
		background-color: transparent;
		width: 100%;
		padding-right: 2rem;
		outline: 0;
		transition: border-color 0.25s;
		grid-column: 1 / -1;
		&:focus {
			border-color: #545454;
			background-color: black;
		}
		&::placeholder {
			color: $translucentColor;
		}
	}

	.chatbox-send {
		color: $textColor;
		border-radius: 0.8rem;
		transition:
			background-color 0.2s,
			color 0.2s,
			box-shadow 0.2s;
		cursor: pointer;
		padding: 0.5rem 1rem;
		margin: 0 0.5rem;
		font: inherit;
		border: 0;
		background: #4142428a;
		grid-column: 2 / 2;
		&:disabled {
			background-color: #4142428a;
			color: $translucentColor;
			cursor: not-allowed;
		}
		&:hover:not(:disabled) {
			background-color: $primaryColor;
		}
	}

	.no-messages {
		padding: 1rem;
		color: $translucentColor;
		text-align: center;
		place-self: center;
		margin: auto 0;
	}

	.bottom-scroller-container {
		position: relative;
		grid-row: 3 / 3;
		grid-column: 1 / -1;
		margin: 0 auto;
	}

	.bottom-scroller {
		position: absolute;
		transform: translateY(-110%) translateX(-50%);
		background-color: #d37c5ca5;
		padding: 0.5rem 1rem;
		text-align: center;
		border-radius: 0.5rem;
		white-space: nowrap;
		font: inherit;
		border: 0;
		outline: 0;
		cursor: pointer;
		color: inherit;
		transition: background-color 0.25s;
		&:hover {
			background-color: $primaryColor;
		}
	}
</style>
