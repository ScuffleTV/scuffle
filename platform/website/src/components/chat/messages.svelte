<script context="module" lang="ts">
	export enum ChatStatus {
		Connecting,
		Disconnected,
		Connected,
	}
</script>

<script lang="ts">
	import { pipe, subscribe, type Subscription } from "wonka";
	import { client } from "$lib/gql";
	import { graphql } from "$gql";
	import { MessageType, type ChatMessage, type User } from "$gql/graphql";
	import { onMount } from "svelte";
	import type { Writable } from "svelte/store";

	export let channelId: string;
	export let onlyUserMessages: boolean = false;

	onMount(() => {
		subscribeToMessages();
		return () => {
			unsubscribeFromMessages();
		};
	});

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

	function intColorToStyle(color?: number) {
		if (!color) return "";
		const r = (color >> 16) & 0xff;
		const g = (color >> 8) & 0xff;
		const b = color & 0xff;
		const a = ((color >> 24) & 0xff) / 255;
		return `color: rgb(${r}, ${g}, ${b}, ${a === 0.0 ? 1.0 : a});`;
	}

	function subscribeToMessages() {
		chatStatus.set(ChatStatus.Connecting);
		const subscriptionQuery = graphql(`
			subscription ChatMessages($channelId: UUID!) {
				chatMessages(channelId: $channelId) {
					id
					type
					content
					author {
						id
						username
						displayName
						displayColor
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
						chatStatus.set(ChatStatus.Connected);
					}
					insertNewMessage(message);
				} else if (response.error) {
					insertNewMessage(createSystemMessage("Failed to connect to chat room"));
				}
			}),
		);
	}

	function unsubscribeFromMessages() {
		chatStatus.set(ChatStatus.Disconnected);
		if (subscription) {
			subscription.unsubscribe();
			insertNewMessage(createSystemMessage("Disconnected from chat"));
		}
	}
</script>

{#if $chatStatus === ChatStatus.Connected}
	{#each messages as message (message.svelteId)}
		<div class="message">
			{#if message.message.type === MessageType.User}
				<span
					><span
						class="message-sender"
						style={intColorToStyle(message.message.author?.displayColor)}
						>{message.message.author?.displayName}</span
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
