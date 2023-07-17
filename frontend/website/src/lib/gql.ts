import {
	createClient,
	fetchExchange,
	cacheExchange,
	subscriptionExchange,
	type Exchange,
	type Client,
	mapExchange,
} from "@urql/svelte";
import { get } from "svelte/store";
import { sessionToken } from "$/store/login";
import {
	createClient as createWsClient,
	type SubscribePayload,
	type Client as WsClient,
} from "graphql-ws";
import { websocketOpen } from "$/store/websocket";
import { env } from "$env/dynamic/public";
import { PUBLIC_GQL_ENDPOINT, PUBLIC_GQL_WS_ENDPOINT } from "$env/static/public";

declare global {
	interface Window {
		SCUFFLE_GQL_CLIENT: Client;
		SCUFFLE_WS_CLIENT: WsClient;
	}
}

const exchanges: Exchange[] = [cacheExchange];

exchanges.push(
	mapExchange({
		onResult(result) {
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

if (typeof window !== "undefined") {
	const wsClient = createWsClient({
		url: PUBLIC_GQL_WS_ENDPOINT,
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

	exchanges.push(
		subscriptionExchange({
			enableAllOperations: true,
			// this allows us to forward subscriptions to the websocket, if it's open otherwise we use the fetch exchange below.
			isSubscriptionOperation: (op) => get(websocketOpen) || op.kind === "subscription",
			forwardSubscription: (operation) => ({
				subscribe: (sink) => ({
					unsubscribe: wsClient.subscribe(operation as SubscribePayload, sink),
				}),
			}),
		}),
	);
}

exchanges.push(fetchExchange);

const gqlURL =
	(typeof window === "undefined" && env.PUBLIC_SSR_GQL_ENDPOINT) || PUBLIC_GQL_ENDPOINT;

// This GQL context is created once and is available to all components.
export const client = createClient({
	// This allows us to change the endpoint at runtime.
	url: gqlURL,
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
