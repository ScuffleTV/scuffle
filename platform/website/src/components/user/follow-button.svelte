<script lang="ts">
	import Fa from "svelte-fa";
	import { faUserCheck, faUserPlus } from "@fortawesome/free-solid-svg-icons";
	import { graphql } from "$/gql";
	import { getContextClient } from "@urql/svelte";
	import { pipe, subscribe, type Subscription } from "wonka";
	import { websocketOpen } from "$/store/websocket";
	import { AuthDialog, authDialog, user } from "$/store/auth";
	import { onDestroy } from "svelte";

	export let channelId: string;
	export let following: boolean;

	const client = getContextClient();

	let subscription: Subscription;

	function sub(channelId: string) {
		subscription?.unsubscribe();
		subscription = pipe(
			client.subscription(
				graphql(`
					subscription UserFollowingSub($channelId: ULID!) {
						userFollowing(channelId: $channelId) {
							channelId
							following
						}
					}
				`),
				{ channelId },
			),
			subscribe((res) => {
				if (res.data) {
					following = res.data.userFollowing.following;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		if ($user) {
			sub(channelId);
		} else {
			subscription?.unsubscribe();
			following = false;
		}
	}

	$: if (!$websocketOpen) {
		subscription?.unsubscribe();
	}

	function onClick() {
		if ($user) {
			client
				.mutation(
					graphql(`
						mutation Follow($channelId: ULID!, $follow: Boolean!) {
							user {
								following: follow(channelId: $channelId, follow: $follow)
							}
						}
					`),
					{ channelId, follow: !following },
				)
				.toPromise()
				.then((res) => {
					if (res.data) {
						following = res.data.user.following;
					}
				});
		} else {
			$authDialog = AuthDialog.Login;
		}
	}

	onDestroy(() => {
		subscription?.unsubscribe();
	});
</script>

<button class="button" class:primary={!following} class:secondary={following} on:click={onClick}>
	<Fa icon={following ? faUserCheck : faUserPlus} />
	<span>{following ? "Following" : "Follow"}</span>
</button>

<style lang="scss">
	.button {
		padding: 0.4rem 0.8rem;
		font-weight: 500;

		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
</style>
