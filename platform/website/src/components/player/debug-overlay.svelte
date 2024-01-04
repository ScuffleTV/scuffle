<script lang="ts">
	import { PUBLIC_EDGE_ENDPOINT, PUBLIC_ORG_ID } from "$env/static/public";
	import type { Player, Variant } from "@scuffle/player";
	import { createEventDispatcher, onMount } from "svelte";

	export let player: Player;
	export let playerToken: string | undefined;
	export let videoEl: HTMLVideoElement;

	const emit = createEventDispatcher();

	let variants: Variant[];
	let currentVariant: number;

	function onManifestLoaded() {
		variants = player.variants;
	}

	function onVariantChange() {
		currentVariant = player.variantId;
	}

	onMount(() => {
		onManifestLoaded();
		onVariantChange();

		player.on("manifestloaded", onManifestLoaded);
		player.on("variant", onVariantChange);

		let interval = setInterval(calcStats, 100);

		return () => {
			player.removeListener("manifestloaded", onManifestLoaded);
			player.removeListener("variant", onVariantChange);
			clearInterval(interval);
		};
	});

	let bufferSize: string;
	let videoTime: string;
	let videoDuration: string;
	let latency: string;
	let resolution: string;
	let bandwidth: string;
	let droppedFrames: string;

	let frameCount: number;
	let lastFrameTime = 0;
	let frameRate: number;

	function calcStats() {
		if (!videoEl) return;
		try {
			if (videoEl.buffered.length) {
				let found = false;
				for (let i = 0; i < videoEl.buffered.length; i++) {
					const start = videoEl.buffered.start(i);
					const end = videoEl.buffered.end(i);
					if (videoEl.currentTime >= start && videoEl.currentTime <= end) {
						bufferSize = (end - videoEl.currentTime).toFixed(3);
						found = true;
						break;
					}
				}

				if (!found) {
					bufferSize = "-1";
				}
			} else {
				bufferSize = "0";
			}

			const currentTime = videoEl.currentTime;
			const duration = videoEl.seekable.length ? videoEl.seekable.end(0) : 0;

			videoTime = currentTime.toFixed(3);
			videoDuration = duration.toFixed(3);
			latency = (duration - currentTime).toFixed(3);

			resolution = `${videoEl.videoWidth}x${videoEl.videoHeight}`;

			const bps = (player.bandwidth || 0) / 1000;

			if (bps > 3000) {
				bandwidth = `${(bps / 1000).toFixed(2)}Mbps`;
			} else {
				bandwidth = `${bps.toFixed(2)}Kbps`;
			}

			const quality = videoEl.getVideoPlaybackQuality();

			droppedFrames = quality.droppedVideoFrames.toString();

			const now = window.performance.now();
			if (now - lastFrameTime >= 1000) {
				frameRate = quality.totalVideoFrames - frameCount;
				frameCount = quality.totalVideoFrames;
				lastFrameTime = now;
			}
		} catch (e) {
			console.error(e);
		}
	}

	function copyPlayerToken() {
		if (playerToken) {
			navigator.clipboard.writeText(playerToken);
		}
	}
</script>

<table class="debug-overlay">
	<thead>
		<tr>
			<th colspan="2">Nerd Overlay</th>
			<button class="close" on:click={() => emit("close")}>[X]</button>
		</tr>
	</thead>
	<br />
	<tr>
		<th>edge endpoint</th>
		<td>{PUBLIC_EDGE_ENDPOINT}</td>
	</tr>
	<tr>
		<th>organization id</th>
		<td>{PUBLIC_ORG_ID}</td>
	</tr>
	<tr>
		<th>room id</th>
		<td>{player.roomId}</td>
	</tr>
	<tr>
		<th>player token</th>
		<td><button on:click={copyPlayerToken}>copy</button></td>
	</tr>
	<br />
	<tr>
		<th>abr</th>
		<td>{player.abrEnabled}</td>
	</tr>
	<tr>
		<th>dvr supported</th>
		<td>{player.dvrSupported}</td>
	</tr>
	<tr>
		<th>dvr</th>
		<td>{player.dvrEnabled}</td>
	</tr>
	<tr>
		<th>realtime</th>
		<td>{player.realtimeMode}</td>
	</tr>
	<tr>
		<th>low latency</th>
		<td>{player.lowLatency}</td>
	</tr>
	<tr>
		<th>visible</th>
		<td>{player.visible}</td>
	</tr>
	<br />
	<tr>
		<th>variants</th>
		<td>
			{#if variants}
				<ul>
					{#each variants as v, index}
						<li class:current={index === currentVariant}>
							{#if v.video_track}
								{v.video_track.name} ({v.video_track.width}x{v.video_track.height})
							{:else}
								audio-only
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				...
			{/if}
		</td>
	</tr>
	<br />
	<tr>
		<th>video time</th>
		<td>{videoTime}</td>
	</tr>
	<tr>
		<th>buffer size</th>
		<td>{bufferSize}</td>
	</tr>
	<tr>
		<th>video duration</th>
		<td>{videoDuration}</td>
	</tr>
	<tr>
		<th>latency</th>
		<td>{latency}</td>
	</tr>
	<tr>
		<th>frame rate</th>
		<td>{frameRate}</td>
	</tr>
	<tr>
		<th>dropped frames</th>
		<td>{droppedFrames}</td>
	</tr>
	<tr>
		<th>resolution</th>
		<td>{resolution}</td>
	</tr>
	<tr>
		<th>bandwidth</th>
		<td>{bandwidth}</td>
	</tr>
</table>

<style lang="scss">
	@import "../../assets/styles/variables.scss";

	.debug-overlay {
		position: absolute;
		top: 1rem;
		left: 1rem;
		padding: 1rem;
		background-color: rgba($bgColor, 0.75);
		font-size: 0.9rem;
		color: $textColor;
		font-weight: 400;

		th {
			text-align: left;
		}

		button {
			color: $textColor;
			padding: 0;
			text-decoration: underline;

			&:hover {
				text-decoration: none;
			}
		}

		ul,
		li {
			margin: 0;
			padding: 0;
		}

		li {
			margin-left: 1rem;

			&.current {
				color: $primaryColor;
			}
		}

		.close {
			color: $textColor;
			font-weight: 700;
		}
	}
</style>
