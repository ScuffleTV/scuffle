<script context="module" lang="ts">
	export enum ChatStatus {
		Connecting,
		Disconnected,
		Connected,
	}
</script>

<script lang="ts">
	import { pipe, subscribe, type Subscription } from "wonka";
	import { graphql } from "$gql";
	import { MessageType, type ChatMessage, type User } from "$gql/graphql";
	import { onDestroy } from "svelte";
	import type { Writable } from "svelte/store";
	import { websocketOpen } from "$/store/websocket";
	import { getContextClient } from "@urql/svelte";

	export let channelId: string;
	export let onlyUserMessages: boolean = false;

	const client = getContextClient();

	interface MessageContainer {
		// Svelte uses this to track changes
		// Without it, svelte uses the index of the array as the key which causes issues when we remove items
		svelteId: number;
		message: ChatMessage;
	}

	export let chatStatus: Writable<ChatStatus>;

	let subscription: Subscription;
	let svelteIdCounter = 1;
	let messages: MessageContainer[] = [];

	$: {
		unsubscribeFromMessages();
		if ($websocketOpen) {
			subscribeToMessages(channelId);
		}
	}

	onDestroy(() => {
		unsubscribeFromMessages();
	});

	$: channelId, (messages = []);

	function createSystemMessage(content: string): ChatMessage {
		return {
			// We cast to User because this is a system message and we don't care about the author or the channel
			channel: {} as User,
			channelId,
			content: content,
			id: "",
			type: MessageType.System,
			user: {} as User,
			userId: "",
		};
	}

	function insertNewMessage(message: ChatMessage) {
		if (onlyUserMessages && message.type !== MessageType.User) return;

		const newMessage: MessageContainer = {
			svelteId: svelteIdCounter,
			message,
		};
		svelteIdCounter++;

		messages = [...messages, newMessage];

		// Show 500 latest messages when scroll is at the bottom
		if (messages.length > 500) {
			messages.shift();
		}
	}

	function colorToStyle(color?: string) {
		if (!color) return "";
		return `color: ${color};`;
	}

	function subscribeToMessages(channelId: string) {
		$chatStatus = ChatStatus.Connecting;
		const subscriptionQuery = graphql(`
			subscription ChatMessages($channelId: ULID!) {
				chatMessages(channelId: $channelId) {
					id
					type
					content
					user {
						id
						username
						displayName
						displayColor {
							color
						}
					}
				}
			}
		`);

		subscription = pipe(
			client.subscription(subscriptionQuery, { channelId }),
			subscribe((response) => {
				const message = response.data?.chatMessages as ChatMessage | undefined;
				if (message) {
					if (message.type === MessageType.Welcome) {
						$chatStatus = ChatStatus.Connected;
					}
					insertNewMessage(message);
				} else if (response.error) {
					insertNewMessage(createSystemMessage("Failed to connect to chat room"));
				}
			}),
		);
	}

	function unsubscribeFromMessages() {
		if (subscription) {
			$chatStatus = ChatStatus.Disconnected;
			subscription.unsubscribe();
			insertNewMessage(createSystemMessage("Disconnected from chat"));
		}
	}
</script>

{#if $chatStatus === ChatStatus.Connected || $chatStatus === ChatStatus.Disconnected}
	{#each messages as message (message.svelteId)}
		<div class="message">
			{#if message.message.type === MessageType.User}
				<span
					><span
						class="message-sender"
						style={colorToStyle(message.message.user?.displayColor.color)}
						>{message.message.user?.displayName}</span
					>:
				</span>
				<span class="message-text">{message.message.content}</span>
			{/if}
			{#if message.message.type === MessageType.Welcome || message.message.type === MessageType.System}
				<span class="message-text info">{message.message.content}</span>
			{/if}
		</div>
	{:else}
		{#if !onlyUserMessages}
			<div class="no-messages">Quiet in here...</div>
		{/if}
	{/each}
{/if}

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.no-messages {
		padding: 1rem;
		color: $textColorLight;
		text-align: center;
		place-self: center;
		margin: auto 0;
	}

	.message {
		padding: 0 0.5rem;

		.info {
			color: $textColorLight;
		}

		.message-sender {
			/* Fallback color */
			color: $primaryColor;
		}

		.message-text {
			overflow-wrap: anywhere;
			word-break: normal;
		}
	}
</style>
