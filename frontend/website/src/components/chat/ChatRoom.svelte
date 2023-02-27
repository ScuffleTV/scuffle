<script lang="ts">
	import { createClient, type Client, type ClientOptions } from "graphql-ws";
	import { onMount } from "svelte";
	import MessageInput from "./MessageInput.svelte";
	import Message from "./Message.svelte";

	export let chatId: number;

	interface ChatMessage {
		username: string;
		content: string;
		metadata: Map<string, string>;
		messageType: string;
	}

	const messageTemplates = new Map<string, ChatMessage>([
		[
			"enter",
			{ username: "", metadata: new Map(), messageType: "info", content: "Connected to chat" },
		],
		["error", { username: "", metadata: new Map(), messageType: "info", content: "Error occured" }],
		[
			"disconnect",
			{ username: "", metadata: new Map(), messageType: "info", content: "Disconnected from chat" },
		],
	]);

	let chatMessages: ChatMessage[] = [];
	let chatStatus = "connecting";

	const subscriptionPayload = {
		query: `
            subscription NewMessage($chatId: Int!) {
                newMessage(chatId: $chatId) {
                    username
                    content
                    metadata
                    messageType
                }
            }
            `,
		variables: { chatId },
	};

	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	function scrollToBottom(node: HTMLElement, _: any) {
		const scroll = () =>
			node.scroll({
				top: node.scrollHeight,
				behavior: "auto",
			});
		scroll();

		return { update: scroll };
	}

	function insertNewMessage(message: ChatMessage) {
		// Show 200 latest messages
		if (message) {
			chatMessages = [...chatMessages, message].slice(-200);
		}
	}

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	function handleResponse(response: any) {
		if (response.data) {
			const { newMessage }: { newMessage?: ChatMessage } = response.data || "";
			if (newMessage) {
				insertNewMessage(newMessage);
			}
		} else {
			insertNewMessage(messageTemplates.get("error")!);
		}
	}

	interface ClientWithOnReconnected extends Client {
		onReconnected(cb: () => void): () => void;
	}

	function createClientWithOnReconnected(options: ClientOptions): ClientWithOnReconnected {
		let abruptlyClosed = false;
		const reconnectedCbs: (() => void)[] = [];

		const client = createClient({
			...options,
			on: {
				...options.on,
				closed: (event) => {
					options.on?.closed?.(event);

					// dont insert connection closed multiple times
					if (chatMessages[chatMessages.length - 1] != messageTemplates.get("disconnect")) {
						insertNewMessage(messageTemplates.get("disconnect")!);
					}

					if ((event as CloseEvent).code !== 1000) {
						abruptlyClosed = true;
						chatStatus = "reconnecting";
					} else {
						chatStatus = "closed";
					}
				},
				connected: (...args) => {
					options.on?.connected?.(...args);
					insertNewMessage(messageTemplates.get("enter")!);
					chatStatus = "connected";

					if (abruptlyClosed) {
						abruptlyClosed = false;
						reconnectedCbs.forEach((cb) => cb());
					}
				},
			},
		});
		return {
			...client,
			onReconnected: (cb) => {
				reconnectedCbs.push(cb);
				return () => {
					reconnectedCbs.splice(reconnectedCbs.indexOf(cb), 1);
				};
			},
		};
	}

	onMount(() => {
		const client = createClientWithOnReconnected({
			url: import.meta.env.VITE_GQL_WS_ENDPOINT,
			retryAttempts: Infinity,
			shouldRetry: () => true,
		});
		client.subscribe(subscriptionPayload, {
			next: (data) => {
				handleResponse(data);
			},
			error: () => {
				insertNewMessage(messageTemplates.get("error")!);
			},
			complete: () => {
				chatStatus = "closed";
				insertNewMessage(messageTemplates.get("disconnect")!);
			},
		});
		client.onReconnected(() => {
			chatStatus = "connected";
		});
	});
</script>

<div class="chat-room">
	<div class="messages" use:scrollToBottom={chatMessages}>
		{#each chatMessages as item}
			<Message username={item.username} message={item.content} messageType={item.messageType} />
		{/each}
	</div>
	<MessageInput {chatStatus} {chatId} handler={handleResponse} />
</div>

<style lang="scss">
	.chat-room {
		height: 100%;
		position: fixed;
		flex-direction: column;
		width: 22rem;
		display: flex;
		border: 1px solid rgb(29, 29, 29);
		background-color: #101415;
		.messages {
			overflow-x: auto;
			display: flex;
			flex-direction: column;
			padding: 0.7rem;
			height: 82%;
			scrollbar-width: thin;
			scrollbar-color: #606364 #0f1314;

			&::-webkit-scrollbar {
				width: 0.25rem;
			}

			&::-webkit-scrollbar-track {
				background: #0f1314;
			}

			&::-webkit-scrollbar-thumb {
				background: #606364;
			}
		}
	}
</style>
