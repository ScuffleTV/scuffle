import {
	createClient,
	dedupExchange,
	fetchExchange,
	cacheExchange,
	subscriptionExchange,
	type Exchange,
	type Client,
	mapExchange,
} from "@urql/svelte";
import { get } from "svelte/store";
import { sessionToken } from "../store/login";
import { createClient as createWsClient, type Client as WsClient } from "graphql-ws";
import { websocketOpen } from "../store/websocket";
import { filter, merge, pipe, share } from "wonka";

declare global {
	interface Window {
		SCUFFLE_GQL_CLIENT: Client;
		SCUFFLE_WS_CLIENT: WsClient;
	}
}

const exchanges: Exchange[] = [dedupExchange, cacheExchange];

if (typeof window !== "undefined") {
	const wsClient = createWsClient({
		url: import.meta.env.VITE_GQL_WS_ENDPOINT,
		lazy: false,
		connectionParams: () => {
			return {
				version: "1.0",
				sessionToken: get(sessionToken)?.token,
			};
		},
		shouldRetry: () => true,
		on: {
			connected: () => websocketOpen.set(true),
			closed: () => websocketOpen.set(false),
		},
	});

	// We need to make sure that the old websocket is closed before we create a new one.
	if (window.SCUFFLE_WS_CLIENT) {
		window.SCUFFLE_WS_CLIENT.dispose();
	}

	window.SCUFFLE_WS_CLIENT = wsClient;

	const wsSink = subscriptionExchange({
		enableAllOperations: true,
		forwardSubscription: (operation) => ({
			subscribe: (sink) => ({
				unsubscribe: wsClient.subscribe(operation, sink),
			}),
		}),
	});

	// This is a bit hard to understand, but it's basically a custom exchange that
	// will send all operations to the websocket if it's open, and to the HTTP
	// endpoint if it's not. Websockets take some time to connect
	// so we dont want to delay the user.

	const operationExchange: Exchange = (input) => {
		const wsExchange = wsSink(input);
		const httpExchange = fetchExchange(input);

		return (ops$) => {
			// We have to share the operations here because if the kind is teardown we forward it to both streams.
			// Teardowns are used to cancel requests, so we need to forward them to both streams.
			const sharedOps$ = pipe(ops$, share);

			// Here we filter if the websocket is open or if the kind is subscription or teardown.
			// We want to use the websocket as much as possible so we send all operations if it is open.
			// We also need to forward all teardowns and subscriptions regardless of the websocket state.
			const wsPipe$ = pipe(
				sharedOps$,
				filter((op) => get(websocketOpen) || op.kind === "subscription" || op.kind === "teardown"),
				wsExchange,
			);

			// Here we use it as a fallback if the websocket is not open, however we still need to forward teardowns even if the websocket is open.
			const httpPipe$ = pipe(
				sharedOps$,
				filter((op) => !get(websocketOpen) || op.kind === "teardown"),
				httpExchange,
			);

			// At the end we need to merge both streams together and return a single result stream.
			return merge([wsPipe$, httpPipe$]);
		};
	};

	// We want to switch out the fetchExchange with our custom exchange.
	exchanges.push(operationExchange);
} else {
	// If we are on the server we just use the fetchExchange. We dont need to worry about websockets.
	exchanges.push(fetchExchange);
}

exchanges.push(
	mapExchange({
		onResult: (result) => {
			// Happens on HTTP requests.
			if (result.error?.networkError?.message === "Unauthorized") {
				// Our token has expired, so we need to log out.
				sessionToken.set(null);
			} else if (result.error) {
				// Check if the error is a InvalidSession error.
				for (const error of result.error.graphQLErrors) {
					if (error.extensions?.kind === "InvalidSession") {
						// Our token has expired, so we need to log out.
						sessionToken.set(null);
						break;
					}
				}
			}

			return result;
		},
	}),
);

// This GQL context is created once and is available to all components.
export const client = createClient({
	url: import.meta.env.VITE_GQL_ENDPOINT,
	exchanges,
	fetchOptions: () => {
		const token = get(sessionToken);
		return {
			headers: token
				? {
						authorization: `Bearer ${token.token}`,
				  }
				: undefined,
		};
	},
});

if (typeof window !== "undefined") {
	window.SCUFFLE_GQL_CLIENT = client;
}
