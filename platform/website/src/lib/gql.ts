import {
	createClient,
	fetchExchange,
	cacheExchange,
	subscriptionExchange,
	type Exchange,
	Client,
} from "@urql/svelte";
import { get } from "svelte/store";
import {
	createClient as createWsClient,
	type SubscribePayload,
	type Client as WsClient,
} from "graphql-ws";
import { websocketOpen } from "$/store/websocket";
import { env } from "$env/dynamic/public";
import { PUBLIC_GQL_ENDPOINT, PUBLIC_GQL_WS_ENDPOINT } from "$env/static/public";
import { browser } from "$app/environment";
import { sessionToken } from "$/store/auth";
import { authExchange } from "@urql/exchange-auth";

declare global {
	interface Window {
		SCUFFLE_WS_CLIENT: WsClient;
	}
}

export function createGqlClient(): Client {
	const exchanges: Exchange[] = [cacheExchange];

	exchanges.push(
		authExchange(async (utils) => {
			return {
				addAuthToOperation(operation) {
					const token = get(sessionToken);
					if (!token) return operation;
					return utils.appendHeaders(operation, {
						Authorization: `Bearer ${token}`,
					});
				},
				didAuthError(error) {
					return error.response?.status === 401 || error.graphQLErrors.some((e) => e.extensions?.kind === "Auth(InvalidToken)");
				},
				async refreshAuth() {
					sessionToken.set(null);
				},
			};
		}),
	);

	// Only when executed in the browser
	if (browser) {
		let wsClient: WsClient;

		sessionToken.subscribe((token) => {
			// We need to make sure that the old websocket is closed before we create a new one.
			if (window.SCUFFLE_WS_CLIENT) {
				window.SCUFFLE_WS_CLIENT.dispose();
			}

			wsClient = createWsClient({
				url: PUBLIC_GQL_WS_ENDPOINT,
				lazy: false,
				connectionParams: () => {
					return {
						version: "1.0",
						sessionToken: token,
					};
				},
				shouldRetry: () => true,
				retryAttempts: 10,
				on: {
					connected: () => {
						console.debug("Connected to websocket");
						websocketOpen.set(true);
					},
					closed: (e) => {
						console.debug("Disconnected from websocket", e);
						websocketOpen.set(false);
						if (
							e instanceof CloseEvent &&
							e.code === 1002 &&
							(e.reason.endsWith("invalid token") || e.reason.endsWith("session expired"))
						) {
							// Our token has expired, so we need to log out.
							sessionToken.set(null);
						}
					},
				},
			});

			window.SCUFFLE_WS_CLIENT = wsClient;
		});

		exchanges.push(
			subscriptionExchange({
				enableAllOperations: true,
				// this allows us to forward subscriptions to the websocket, if it's open otherwise we use the fetch exchange below.
				isSubscriptionOperation: (op) =>
					op.kind === "subscription" || (get(websocketOpen) && !!wsClient),
				forwardSubscription: (operation) => ({
					subscribe: (sink) => ({
						unsubscribe: wsClient.subscribe(operation as SubscribePayload, sink),
					}),
				}),
			}),
		);
	}

	exchanges.push(fetchExchange);

	const gqlURL = browser ? PUBLIC_GQL_ENDPOINT : env.PUBLIC_SSR_GQL_ENDPOINT || PUBLIC_GQL_ENDPOINT;

	// This GQL context is created once and is available to all components.
	return createClient({
		// This allows us to change the endpoint at runtime.
		url: gqlURL,
		exchanges,
	});
}
