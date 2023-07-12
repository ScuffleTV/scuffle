<script lang="ts">
	import { client } from "$lib/gql";
	import { user } from "$store/user";
	import { graphql } from "$gql";
	import { browser } from "$app/environment";
	import { afterUpdate, onMount } from "svelte";
	import Fa from "svelte-fa";
	import { faChevronLeft } from "@fortawesome/free-solid-svg-icons";
	import { faFaceSmile } from "@fortawesome/free-regular-svg-icons";
	import Send from "../icons/send.svelte";
	import Messages, { ChatStatus } from "./messages.svelte";
	import { writable } from "svelte/store";

	const MAX_MESSAGE_LENGTH = 500;

	let collapsed: boolean = browser && window.localStorage.getItem("chatroom_collapsed") === "true";
	$: {
		if (browser) {
			window.localStorage.setItem("chatroom_collapsed", JSON.stringify(collapsed));
		}
	}

	export let channelId: string;

	function collapseNav() {
		collapsed = !collapsed;
	}

	let messagesEl: HTMLDivElement;
	let chatStatus = writable(ChatStatus.Connecting);
	let chatMessage = "";
	let atBottom = true;

	$: inputEmpty = chatMessage.length === 0 || chatMessage === "\n";

	$: inputValid = chatMessage.length <= MAX_MESSAGE_LENGTH;

	$: sendEnabled = inputValid && !inputEmpty && $chatStatus === ChatStatus.Connected && $user;

	$: chatError =
		$user === null
			? "Please create an account to chat"
			: chatMessage.length > MAX_MESSAGE_LENGTH
			? `Your message is longer than ${MAX_MESSAGE_LENGTH} characters`
			: null;

	function onScroll() {
		atBottom = messagesEl.scrollTop + messagesEl.offsetHeight >= messagesEl.scrollHeight - 30;
	}

	onMount(() => {
		// Scroll to bottom
		messagesEl.addEventListener("scroll", onScroll);
		return () => {
			if (messagesEl) {
				messagesEl.removeEventListener("scroll", onScroll);
			}
		};
	});

	function scrollToBottom() {
		// We set at bottom to true so that if new messages are added we scroll to the bottom again
		atBottom = true;
		// We request an animation frame to make sure the scroll happens after the DOM has been updated
		window.requestAnimationFrame(() => {
			if (messagesEl) {
				messagesEl.scrollTop = messagesEl.scrollHeight;
			}
		});
	}

	afterUpdate(() => {
		if (atBottom) {
			scrollToBottom();
		}
	});

	const newMessageQuery = graphql(`
		mutation SendMessage($channelId: UUID!, $content: String!) {
			chat {
				sendMessage(channelId: $channelId, content: $content) {
					id
				}
			}
		}
	`);

	async function sendMessageInner(message: string) {
		const response = await client
			.mutation(newMessageQuery, { channelId: channelId, content: message })
			.toPromise();
		if (response.error) {
			// TODO: Failed to send message
		}
	}

	function onChatmessageKeydown(e: KeyboardEvent) {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			sendMessage();
		}
		e.stopPropagation();
	}

	// Remove as soon as widely supported
	// https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/contenteditable#browser_compatibility
	// https://caniuse.com/mdn-html_global_attributes_contenteditable_plaintext-only
	function onChatmessagePaste(e: ClipboardEvent) {
		// To prevent inserting rich text
		if (e.clipboardData) {
			const text = e.clipboardData.getData("text/plain");
			// This is deprecated but it has no good alternative.
			// https://developer.mozilla.org/en-US/docs/Web/API/Document/execCommand
			// https://stackoverflow.com/a/70831583/10772729
			// TL;DR: This will most likely be never removed from any major browser
			document.execCommand("insertText", false, text);
			// When execCommand is removed or is unsupported,
			// it will fall back to the default rich text paste, because an error will occur before preventDefault is called.
			e.preventDefault();
		}
	}

	function sendMessage() {
		if (sendEnabled) {
			sendMessageInner(chatMessage);
			chatMessage = "";
		}
	}
</script>

{#if collapsed}
	<div class="uncollapse">
		<button class="collapse-icon" on:click={collapseNav}>
			<Fa icon={faChevronLeft} fw />
		</button>
	</div>
{:else}
	<div class="chatroom" class:collapsed>
		<div class="top">
			<div>
				<button class="collapse-icon" on:click={collapseNav}>
					<Fa icon={faChevronLeft} fw />
				</button>
				<span class="chat-title">Chat</span>
			</div>
			<span
				class="connection-indicator"
				class:connecting={$chatStatus === ChatStatus.Connecting}
				class:disconnected={$chatStatus === ChatStatus.Disconnected}
				class:connected={$chatStatus === ChatStatus.Connected}
			>
				{#if $chatStatus === ChatStatus.Connecting}
					Connecting...
				{:else if $chatStatus === ChatStatus.Disconnected}
					Disconnected
				{:else if $chatStatus === ChatStatus.Connected}
					Connected
				{/if}
			</span>
		</div>
		<div class="messages" bind:this={messagesEl}>
			<Messages {channelId} {chatStatus} />
		</div>
		{#if atBottom === false}
			<div class="bottom-scroller-container">
				<button class="bottom-scroller" on:click={scrollToBottom}> Scroll to bottom </button>
			</div>
		{/if}
		<form class="chatbox" on:submit|preventDefault={sendMessage}>
			<div class="chatbox-input">
				<div
					class="input"
					role="textbox"
					tabindex="0"
					bind:innerText={chatMessage}
					on:keydown={onChatmessageKeydown}
					on:paste={onChatmessagePaste}
					contenteditable="true"
					class:invalid={!inputValid}
				/>
				<span class="placeholder" class:hidden={!inputEmpty}>SEND A MESSAGE</span>
				<button type="button">
					<Fa icon={faFaceSmile} size="1.5x" />
				</button>
			</div>
			<div class="below-input">
				<span class="error" class:shown={chatError && !inputEmpty}>{chatError}</span>
				<button class="button primary chatbox-send" type="submit" disabled={!sendEnabled}>
					<span>Send</span><Send />
				</button>
			</div>
		</form>
	</div>
{/if}

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.uncollapse {
		position: absolute;
		top: 0;
		right: 0;
		z-index: 6;
		transform: rotate(0deg) translateY(calc($topNavHeight + 1rem)) translateX(-1rem);
	}

	.chatroom {
		grid-area: chat;
		overflow-y: auto;

		background-color: $bgColor2;
		min-width: $chatroomWidth;
		width: $chatroomWidth;

		display: flex;
		flex-direction: column;

		border-left: 0.125rem solid $borderColor;
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
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;

		padding: 1rem 0.5rem;

		& > div {
			display: flex;
			align-items: center;
			gap: 0.5rem;
		}

		.connection-indicator {
			&.connecting {
				color: $loadingColor;
				&::before {
					background-color: $loadingColor;
				}
			}

			&.disconnected {
				color: $errorColor;
				&::before {
					background-color: $errorColor;
				}
			}

			&.connected {
				color: $successColor;
				&::before {
					background-color: $successColor;
				}
			}

			&::before {
				content: "";
				display: inline-block;
				width: 0.4rem;
				height: 0.4rem;
				border-radius: 50%;
				margin-right: 0.4rem;
				margin-bottom: 0.1rem;
			}
		}
	}

	.collapse-icon {
		display: flex;
		place-items: center;
		border: 0;
		outline: 0;
		background-color: transparent;
		padding: 0;
		cursor: pointer;
		transition: color 0.25s;
		color: $textColorLight;
		font-size: 1.2rem;
		grid-row: 1 / 1;

		&:hover,
		&:focus-visible {
			color: $textColor;
		}
	}

	.messages {
		flex-grow: 1;
		overflow-y: scroll;
		overflow-x: hidden;

		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.chatbox {
		padding: 0.5rem;

		background-color: $bgColor2;
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.5rem;
	}

	.chatbox-input {
		position: relative;

		display: flex;
		align-items: center;

		& > .input {
			max-height: 10rem;
			width: 100%;
			font: inherit;
			color: $textColor;
			word-break: break-all;
			overflow-y: scroll;

			background-color: $bgColor2;
			padding: 0.75rem 1rem;
			padding-right: 2.75rem;
			outline: none;

			border: 1px solid $borderColor;
			border-radius: 0.5rem;

			transition: border-color 0.25s;

			&:focus {
				border-color: $textColorDark;
				background-color: black;
			}

			&.invalid {
				border-color: $errorColor;
			}
		}

		& > .placeholder {
			font: inherit;
			font-weight: 500;
			position: absolute;
			left: 1rem;
			top: 0;
			bottom: 0;
			pointer-events: none;

			display: flex;
			align-items: center;

			color: $textColorLight;
			opacity: 0.5;

			&.hidden {
				visibility: hidden;
			}
		}

		& > button {
			position: absolute;
			right: 0.5rem;
			padding: 0.5rem;

			background-color: transparent;
			border: none;
			color: $textColor;
			transition: background-color 0.25s;

			border-radius: 50%;

			&:hover,
			&:focus-visible {
				background-color: $bgColorLight;
			}
		}
	}

	.below-input {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;

		.error {
			font: inherit;
			font-weight: 500;
			color: $errorColor;
			visibility: hidden;

			&.shown {
				visibility: visible;
			}
		}

		.chatbox-send {
			padding: 0.5rem 1rem;
			font: inherit;
			font-weight: 500;

			display: flex;
			align-items: center;
			gap: 0.5rem;
		}
	}

	.bottom-scroller-container {
		position: relative;
		margin: 0 auto;
	}

	.bottom-scroller {
		position: absolute;
		transform: translateY(-110%) translateX(-50%);
		background-color: rgba($primaryColor, 0.6);
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

		&:hover,
		&:focus-visible {
			background-color: $primaryColor;
		}
	}
</style>
