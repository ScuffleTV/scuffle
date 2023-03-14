<script lang="ts">
	import CloseSidebar from "./closeSidebar.svelte";
	import { user } from "../store/user";
	import { onMount } from "svelte";

	export let collapsed = false;

	function collapseNav() {
		collapsed = !collapsed;
	}

	interface Message {
		// Svelte uses this to track changes
		// Without it, svelte uses the index of the array as the key which causes issues when we remove items
		svelte_id: number;
		text: string;
		sender: string;
		color: string;
	}

	let messages: Message[] = [];

	let chatMessage = "";
	let svelte_id = 1;
	let messagesEl: HTMLDivElement;

	$: valid = chatMessage.length > 0 && $user !== null;
	let atBottom = true;

	function onScroll() {
		atBottom = messagesEl.scrollTop + messagesEl.offsetHeight >= messagesEl.scrollHeight - 30;
	}

	onMount(() => {
		// Scroll to bottom
		messagesEl.addEventListener("scroll", onScroll);
		return () => {
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

	function sendMessageInner(username: string, message: string) {
		messages.push({
			svelte_id, // Svelte uses this to track changes
			sender: username,
			text: message,
			color: "teal",
		});

		svelte_id++;

		// Svite doesn't detect changes to arrays, so we have to reassign it to trigger a re-render
		messages = messages;

		// Not sure if this is the best way to do this, but it works
		if (messages.length > 500) {
			messages = messages.splice(1);
		}

		// Scroll to bottom
		if (atBottom) {
			scrollToBottom();
		}
	}

	function sendMessage() {
		if (valid && $user) {
			sendMessageInner($user.username, chatMessage);
			chatMessage = "";
		}
	}

	console.log((username: string, message: string) => {
		sendMessageInner(username, message);
	});
</script>

{#if collapsed}
	<div class="uncollapse">
		<button class="collapse-icon" on:click={collapseNav}>
			<CloseSidebar />
		</button>
	</div>
{/if}

<div class="chatroom" class:collapsed>
	<div class="top">
		<button class="collapse-icon" on:click={collapseNav}>
			<CloseSidebar />
		</button>
		<span class="chat-title">Chat</span>
	</div>
	<div class="messages" bind:this={messagesEl}>
		{#each messages as message (message.svelte_id)}
			<div class="message" class:odd={message.svelte_id % 2}>
				<span class="message-sender" style={`color: ${message.color}`}>{message.sender}:</span>
				<span class="message-text">{message.text}</span>
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
			placeholder="Type a message..."
			bind:value={chatMessage}
			required
		/>
		<input class="chatbox-send" type="submit" value="Send" disabled={!valid} />
	</form>
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

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
		color: #ffffff;
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
		color: #ffffff70;
		font-size: 1.75rem;
		grid-row: 1 / 1;
		&:hover {
			color: #f79986;
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
		padding: 0.75rem 1rem;
		gap: 0.5rem;
		word-break: break-all;
		&.odd {
			background-color: #4142428a;
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
		color: white;
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
			color: #ffffff70;
		}
	}

	.chatbox-send {
		color: white;
		border-radius: 0.8rem;
		transition: background-color 0.2s, color 0.2s, box-shadow 0.2s;
		cursor: pointer;
		padding: 0.5rem 1rem;
		margin: 0 0.5rem;
		font: inherit;
		border: 0;
		background: #4142428a;
		grid-column: 2 / 2;
		&:disabled {
			background-color: #4142428a;
			color: #ffffff70;
			cursor: not-allowed;
		}
		&:hover:not(:disabled) {
			background-color: #ff7357;
		}
	}

	.no-messages {
		padding: 1rem;
		color: #ffffff70;
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
			background-color: #ff7357;
		}
	}
</style>
