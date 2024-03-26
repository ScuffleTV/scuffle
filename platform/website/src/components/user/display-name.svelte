<script lang="ts">
	import { graphql } from "$/gql";
	import { websocketOpen } from "$/store/websocket";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { type Subscription, subscribe, pipe } from "wonka";

	export let userId: string;
	export let displayName: string;

	let subscription: Subscription;

	const client = getContextClient();

	function sub(userId: string) {
		subscription?.unsubscribe();
		subscription = pipe(
			client.subscription(
				graphql(`
					subscription DisplayName($userId: ULID!) {
						userDisplayName(userId: $userId) {
							displayName
						}
					}
				`),
				{ userId },
			),
			subscribe((res) => {
				if (res.data) {
					displayName = res.data.userDisplayName.displayName;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		sub(userId);
	}

	$: if (!$websocketOpen) {
		subscription?.unsubscribe();
	}

	onDestroy(() => {
		subscription?.unsubscribe();
	});
</script>

{displayName}
