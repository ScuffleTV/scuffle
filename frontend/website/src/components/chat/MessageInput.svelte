<script lang="ts">
	import { getContextClient } from "@urql/svelte";
	import { loginMode, sessionToken } from "../../store/login";
	import Fa from "svelte-fa";
	import { faCircleNotch } from "@fortawesome/free-solid-svg-icons";

	export let chatId: number;
	export let chatStatus: string;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	export let handler: (response: any) => void;

	let currentMessage = "";
	let messageStatus = "";

	const client = getContextClient();
	const query = `
            mutation SendMessage($chatId: Int!, $content: String!) {
                chat {
                    sendMessage(chatId: $chatId, content: $content)
                }
            }
        `;

	async function handleSubmit() {
		if (!$sessionToken) {
			loginMode.set(1);
		} else if (
			chatStatus === "connected" &&
			currentMessage.length > 0 &&
			messageStatus !== "loading"
		) {
			messageStatus = "loading";
			const response = await client
				.mutation(query, { chatId, content: currentMessage })
				.toPromise();
			handler(response);
			messageStatus = "";
			currentMessage = "";
		}
	}

	async function handleEnterKey(event: KeyboardEvent) {
		if (event.key === "Enter") {
			handleSubmit();
		}
	}
</script>

<div class="chat-message-input">
	<div class="input-container">
		<div class="input-error">
			{#if chatStatus == "reconnecting"}
				<Fa icon={faCircleNotch} spin />
				<p class="left-margin">Reconnecting</p>
			{:else if chatStatus == "connecting"}
				<Fa icon={faCircleNotch} spin />
				<p class="left-margin">Connecting to chat.</p>
			{:else if currentMessage.length >= 500}
				<p>Message can't exceed 500 characters.</p>
			{/if}
		</div>
		<textarea
			on:keydown={handleEnterKey}
			bind:value={currentMessage}
			maxlength="500"
			placeholder="Send Message"
			class="input-field"
		/>
	</div>
	<button on:click={handleSubmit}>
		{#if messageStatus === "loading"}
			<Fa icon={faCircleNotch} spin />
		{:else}
			Send
		{/if}
	</button>
</div>

<style lang="scss">
	.chat-message-input {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		height: 18%;
		padding: 0.7rem;
		.input-container {
			position: relative;
			width: 100%;
			height: 70%;
			display: flex;
			flex-direction: column;
			margin-bottom: 15px;
		}
		.input-error {
			width: 100%;
			color: #878787;
			font-size: 0.8rem;
			height: 40px;
			display: flex;
			align-items: center;
			padding: 7px 7px;
			border-top-right-radius: 5px;
			border-top-left-radius: 5px;
			&:has(p) {
				border: 1px solid rgb(29, 29, 29);
				background-color: #0b0b0b;
			}
			p {
				margin: 0;
				&.left-margin {
					margin-left: 10px;
				}
			}
		}
		.input-field {
			height: 60%;
			background-color: #1b2324;
			width: 100%;
			border: 0;
			border-radius: 3px;
			padding: 8px;
			font-family: inherit;
			color: white;
			resize: none;
			font-size: 0.8rem;
			outline: 0;
			border: 1px solid #192021;
			transition: border 0.3s;
			scrollbar-width: thin;
			scrollbar-color: #606364 #0f1314;

			&:focus,
			&:focus-visible {
				border: 1px solid #ff7357;
			}

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
		button {
			font-weight: bold;
			color: white;
			background-color: #ff7357;
			border: 0;
			cursor: pointer;
			width: 30%;
			padding: 6px 4px;
			border-radius: 3px;
			transition: background-color 0.5s;
			&:hover {
				background-color: #ff7e65;
			}
		}
	}
</style>
