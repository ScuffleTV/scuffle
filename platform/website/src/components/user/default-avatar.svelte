<script lang="ts">
	import { graphql } from "$/gql";
	import { pipe, subscribe, type Subscription } from "wonka";
	import type { DisplayColor } from "$/gql/graphql";
	import { getContextClient } from "@urql/svelte";
	import { onDestroy } from "svelte";
	import { websocketOpen } from "$/store/websocket";

	export let userId: string;
	export let displayColor: DisplayColor;
	export let size: number = 48;

	const client = getContextClient();

	let sub: Subscription;

	function subToDisplayColor(userId: string) {
		sub?.unsubscribe();
		sub = pipe(
			client.subscription(
				graphql(`
					subscription DisplayColor($userId: ULID!) {
						userDisplayColor(userId: $userId) {
							displayColor {
								rgb
								hsl {
									h
									s
									l
								}
								isGray
							}
						}
					}
				`),
				{ userId },
			),
			subscribe(({ data }) => {
				if (data) {
					displayColor = data.userDisplayColor.displayColor;
				}
			}),
		);
	}

	$: if ($websocketOpen) {
		subToDisplayColor(userId);
	}

	$: if (!$websocketOpen) {
		sub?.unsubscribe();
	}

	onDestroy(() => {
		sub?.unsubscribe();
	});
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 52 52"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	style="--hue: {displayColor.hsl.h}"
>
	<circle cx="26" cy="26" r="26" fill="url(#linear-gradient-{userId})" />
	<path
		d="M26.1359 36.552C26.1359 36.552 39.7004 37.2585 42.3851 34.5739C45.0697 31.9363 42.1967 25.1069 42.1967 25.1069C42.1967 25.1069 44.0807 17.0059 43.6097 16.0639C43.1387 15.1219 33.7188 16.3465 33.7188 16.3465C33.7188 16.3465 27.5489 16.9117 26.1359 17.1943C24.77 16.9117 18.5529 16.3465 18.5529 16.3465C18.5529 16.3465 9.13311 15.0748 8.66212 16.0639C8.19113 17.053 10.1222 25.1069 10.1222 25.1069C10.1222 25.1069 7.24914 31.9363 9.93379 34.5739C12.5713 37.2585 26.1359 36.552 26.1359 36.552Z"
		fill="white"
	/>
	<path
		d="M26.0029 16.6111C12.0144 16.6111 10.114 31.8889 9.93257 34.5738C11.0629 35.7984 14.6743 36.3332 15.2077 35.4687C16.5735 33.2551 16.5264 26.8025 26.1347 26.8025C35.7429 26.8025 35.6958 33.2551 37.1088 35.4687C37.8038 36.5576 41.2535 35.7984 42.3839 34.5738C41.8913 32.5 40.0384 16.6111 26.0029 16.6111Z"
		fill="#FF7357"
	/>
	<path
		d="M23.7807 35.1386C23.7807 33.8669 24.864 32.7837 26.1357 32.7837C27.4544 32.7837 28.5377 33.8198 28.5377 35.1386C27.9254 35.8922 26.1357 36.5516 26.1357 36.5516C26.1357 36.5516 24.5343 35.9393 23.7807 35.1386ZM21.4729 22.7516C17.2339 21.7625 11.3936 33.7727 13.3718 35.3741C13.5131 35.5154 22.4619 27.0376 22.7445 26.3311C23.0742 24.3529 22.7445 23.1283 21.4729 22.7516ZM30.8456 22.7516C35.0374 21.7625 40.9248 33.7727 38.9466 35.3741C38.7582 35.5154 29.8094 27.0376 29.5739 26.3311C29.2442 24.3529 29.5739 23.1283 30.8456 22.7516ZM33.7657 16.299C34.8019 15.9222 41.0661 15.0744 42.7146 14.886C43.7507 14.7918 44.7398 15.5454 44.4572 16.5816L42.2436 25.0594C42.1494 25.342 41.0661 24.0703 41.2074 23.3167L42.8088 16.9113C42.9501 16.3932 42.4791 16.3932 41.7726 16.4403L35.2729 17.1939C34.7077 17.2881 33.436 16.4403 33.7657 16.299ZM18.5527 16.299C17.5165 15.9222 11.2523 15.0744 9.60388 14.886C8.5677 14.7918 7.57862 15.5454 7.86121 16.5816C8.00251 17.1939 9.65098 23.7877 10.0749 25.0594C10.1691 25.342 11.2523 24.0703 11.064 23.3167L9.46258 16.9113C9.32129 16.3932 9.83938 16.3932 10.5459 16.4403L17.0455 17.1939C17.6107 17.2881 18.8824 16.4403 18.5527 16.299Z"
		fill="#2B3C3F"
	/>
	<defs>
		<linearGradient
			id="linear-gradient-{userId}"
			x1="26"
			y1="0"
			x2="26"
			y2="52"
			gradientUnits="userSpaceOnUse"
		>
			<stop
				stop-color="currentColor"
				style={`color: ${displayColor.isGray ? "hsl(0 0% 70%)" : "hsl(var(--hue) 100% 90%)"}`}
			/>
			<stop
				offset="1"
				stop-color="currentColor"
				style={`color: ${displayColor.isGray ? "hsl(0 0% 40%)" : "hsl(var(--hue) 90% 60%)"}`}
			/>
		</linearGradient>
	</defs>
</svg>

<style lang="scss">
	linearGradient {
		stop {
			transition: color 0.2s cubic-bezier(0, 0, 0.8, 0);
		}
	}
</style>
