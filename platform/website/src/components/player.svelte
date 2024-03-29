<script lang="ts">
	import { onDestroy, onMount, tick } from "svelte";
	import init, { Player, type EventError, type Variant } from "@scuffle/player";
	import Play from "$/components/icons/player/play.svelte";
	import Pause from "$/components/icons/player/pause.svelte";
	import Volume from "$/components/icons/player/volume.svelte";
	import EnterPictureInPicture from "$/components/icons/player/enter-picture-in-picture.svelte";
	import ExitPictureInPicture from "$/components/icons/player/exit-picture-in-picture.svelte";
	import EnterTheaterMode from "./icons/player/enter-theater-mode.svelte";
	import ExitTheaterMode from "./icons/player/exit-theater-mode.svelte";
	import FullscreenMaximize from "$/components/icons/player/fullscreen-maximize.svelte";
	import FullscreenMinimize from "$/components/icons/player/fullscreen-minimize.svelte";
	import Clip from "$/components/icons/player/clip.svelte";
	import Lightning from "./icons/player/lightning.svelte";
	import Spinner from "./player/spinner.svelte";
	import { sideNavHidden, topNavHidden } from "$/store/layout";
	import { authDialog } from "$/store/auth";
	import { dev } from "$app/environment";
	import DebugOverlay from "./player/debug-overlay.svelte";
	import type { ChannelLive } from "$/gql/graphql";

	function loadVolume() {
		const storedVolume = localStorage.getItem("player_volume");
		return storedVolume ? parseFloat(storedVolume) : null;
	}

	function loadTheaterMode() {
		return localStorage.getItem("player_theaterMode") === "true";
	}

	function loadBandwithEstimate() {
		const storedEstimate = localStorage.getItem("player_bandwidthEstimate");
		return storedEstimate ? parseFloat(storedEstimate) : null;
	}

	function storeBandwithEstimate() {
		if (player?.bandwidth) {
			localStorage.setItem("player_bandwidthEstimate", player.bandwidth.toString());
		}
	}

	function friendlyVariantName(variant: Variant) {
		if (variant.video_track) {
			return `${variant.video_track.height}p${variant.video_track.frame_rate}`;
		} else {
			return "audio only";
		}
	}

	function autoVariantName(selected: number, current: number) {
		let name = "auto";
		if (selected === -1 && player?.variants) {
			const variant = player.variants.at(current);
			if (variant) {
				name += ` (${friendlyVariantName(variant)})`;
			}
		}
		return name;
	}

	// export let edgeEndpoint: string;
	// export let organizationId: string;
	// export let roomId: string;
	// export let playerToken: string | undefined = undefined;
	export let live: ChannelLive;

	export let controls = true;
	export let showPip = true;
	export let showTheater = true;
	export let initMuted = false;

	let playerEl: HTMLDivElement;
	let videoEl: HTMLVideoElement;

	enum PlayerState {
		Loading,
		Playing,
		Paused,
		Error,
	}

	let player: Player;
	let state = PlayerState.Loading;
	let variants: Variant[];
	let currentVariant: number;

	// This is only used for hiding the controls when the mouse is not moving anymore
	let controlsHidden = false;
	let controlsHiddenTimeout: NodeJS.Timeout | number;

	let theaterMode = loadTheaterMode();
	let pip = false;
	let fullscreen = false;
	let audioOnly = false;
	let selectedVariant: number;
	let volume = initMuted ? 0.0 : loadVolume() ?? 1.0;

	let debugOverlay = false;

	$: localStorage.setItem("player_theaterMode", theaterMode.toString());
	$: localStorage.setItem("player_volume", volume.toString());

	$: {
		$topNavHidden = theaterMode;
		$sideNavHidden = theaterMode;
	}

	function pipEnabled() {
		pip = true;
	}

	function pipDisabled() {
		pip = false;
	}

	function playVideoEl() {
		if (!videoEl) return;
		videoEl.play().catch(() => {
			// Play failed, probably because of autoplay restrictions
			// Mute the video and try again
			console.debug("[player] play failed, trying again muted");
			volume = 0.0;
			// Wait for one svelte tick to make sure the volume is set before trying again
			tick().then(() => {
				if (!videoEl) return;
				videoEl.play().catch((e) => {
					console.error("[player] failed to play video", e);
					// Setting state to paused so the user knows that they have to click play
					state = PlayerState.Paused;
				});
			});
		});
	}

	onMount(() => {
		document.addEventListener("enterpictureinpicture", pipEnabled);
		document.addEventListener("leavepictureinpicture", pipDisabled);
		return () => {
			document.removeEventListener("enterpictureinpicture", pipEnabled);
			document.removeEventListener("leavepictureinpicture", pipDisabled);
		};
	});

	$: {
		if (player) {
			if (selectedVariant === -1) {
				player.abrEnabled = true;
			} else {
				player.abrEnabled = false;
				player.variantId = selectedVariant;
			}
		}
	}

	function onManifestLoaded() {
		console.debug("[player] manifest loaded, variants: ", player.variants);
		variants = player.variants;
	}

	function onVariantChange() {
		let variant = player.variants.at(player.variantId);
		if (variant) {
			currentVariant = player.variantId;
			console.debug(`[player] switched to variant ${variant.video_track?.name ?? "audio only"}`);
			audioOnly = !variant.video_track;
		} else {
			console.debug("[player] switched to unkonwn variant");
		}
	}

	function onError(evt: EventError) {
		state = PlayerState.Error;
		console.error(evt);
	}

	function onDestoyed() {
		console.debug("[player] destroyed");
		if (videoEl) {
			videoEl.pause();
		}
	}

	function onStarted() {
		console.debug("[player] started");
		if (player?.realtimeMode) {
			player.toRealtime();
		}
	}

	function onStopped() {
		console.debug("[player] stopped");
		videoEl?.pause();
	}

	onMount(() => {
		init().then(() => {
			player = new Player(videoEl, {
				server: live.edgeEndpoint,
				organization_id: live.organizationId,
				abr_default_bandwidth: loadBandwithEstimate() ?? undefined,
			});
			player.loadRoom(live.roomId, live.playerToken ?? undefined);

			player.on("manifestloaded", onManifestLoaded);
			player.on("variant", onVariantChange);
			player.on("error", onError);
			player.on("destroyed", onDestoyed);
			player.on("started", onStarted);
			player.on("stopped", onStopped);
			player.on("finished", () => {
				console.debug("[player] finished");
			});

			player.start();
			playVideoEl();
			console.debug("[player] initialized");
		});

		const interval = setInterval(storeBandwithEstimate, 5000);
		return () => {
			clearInterval(interval);
		};
	});

	onDestroy(() => {
		player?.destroy();
	});

	// When the player is clicked
	function onPlayerClick(e: MouseEvent) {
		if (!controls) return;
		if (e.button === 0) {
			onPlayClick();
		}
	}

	function onPlayerDblClick(e: MouseEvent) {
		if (!controls) return;
		if (e.button === 0) {
			toggleFullscreen();
		}
	}

	// When the player button is clicked
	function onPlayClick() {
		switch (state) {
			case PlayerState.Playing:
				console.debug("[player] stopping (user interaction)");
				player?.stop();
				break;
			case PlayerState.Loading:
			case PlayerState.Paused:
				console.debug("[player] starting (user interaction)");
				player?.start();
				playVideoEl();
				break;
		}
	}

	function onMouseMove() {
		controlsHidden = false;
		clearTimeout(controlsHiddenTimeout);
		controlsHiddenTimeout = setTimeout(() => {
			controlsHidden = true;
		}, 2000);
	}

	function toggleMuted() {
		if (volume === 0.0) {
			volume = 1.0;
		} else {
			volume = 0.0;
		}
	}

	function togglePictureInPicture() {
		if (document.pictureInPictureElement) {
			document.exitPictureInPicture();
		} else {
			videoEl.requestPictureInPicture();
		}
	}

	function onClipClick() {
		console.log("clip it!");
	}

	function toggleTheaterMode() {
		theaterMode = !theaterMode;
	}

	function toggleFullscreen() {
		if (document.fullscreenElement) {
			document.exitFullscreen();
		} else {
			playerEl.requestFullscreen();
		}
	}

	// Attention: This is a global event handler since it is addded on body!
	function onKeyDown(e: KeyboardEvent) {
		// Ignore if in any kind of login window
		if ($authDialog.opened) return;
		// Ignore if the key is held down
		if (e.repeat) return;
		// Ignore if controls disabled
		if (!controls) return;
		switch (e.key) {
			case " ":
			case "k":
				onPlayClick();
				break;
			case "f":
				toggleFullscreen();
				break;
			case "m":
				toggleMuted();
				break;
			case "p":
				// Ignore if pip mode is not allowed
				if (!showPip) return;
				togglePictureInPicture();
				break;
			case "t":
				// Ignore if theater mode is not allowed
				if (!showTheater) return;
				toggleTheaterMode();
				break;
			case "d":
				debugOverlay = !debugOverlay;
				break;
			default:
				return;
		}
		e.preventDefault();
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<div
	class="player"
	bind:this={playerEl}
	class:theater-mode={theaterMode}
	class:controls-hidden={controlsHidden}
	on:mousemove={onMouseMove}
	on:fullscreenchange={() => (fullscreen = document.fullscreenElement !== null)}
	role="none"
	class:dev
>
	<video
		bind:this={videoEl}
		on:playing={() => (state = PlayerState.Playing)}
		on:play={() => (state = PlayerState.Playing)}
		on:pause={() => (state = PlayerState.Paused)}
		on:waiting={() => (state = PlayerState.Loading)}
		on:stalled={() => (state = PlayerState.Loading)}
		on:ended={() => (state = PlayerState.Paused)}
		on:error={() => (state = PlayerState.Error)}
		on:click={onPlayerClick}
		on:dblclick={onPlayerDblClick}
		preload="metadata"
		autoplay
		class:darken={state === PlayerState.Paused || state === PlayerState.Error || audioOnly}
		bind:volume
		muted={volume === 0.0}
	>
		<!-- No captions, this must be specified explicitly to suppress an a11y warning -->
		<track kind="captions" />
		<span>Sorry, your browser can't play this</span>
	</video>
	<div class="center-icons" class:show={state !== PlayerState.Playing || audioOnly}>
		{#if state === PlayerState.Loading}
			<Spinner />
		{:else if state === PlayerState.Playing}
			{#if audioOnly}
				<Volume volume={1.0} size={64} />
				<span>Audio Only</span>
			{:else}
				<Play size={64} />
			{/if}
		{:else if state === PlayerState.Paused}
			<Pause size={64} />
		{:else if state === PlayerState.Error}
			<Lightning />
			<span>Something went wrong</span>
		{/if}
	</div>
	{#if controls}
		<div class="controls" class:hidden={controlsHidden}>
			<div>
				<button
					title={state === PlayerState.Playing ? "Pause" : "Play"}
					on:click|preventDefault={onPlayClick}
					disabled={state === PlayerState.Error}
				>
					{#if state === PlayerState.Playing}
						<Pause />
					{:else}
						<Play />
					{/if}
				</button>
				<button
					class="live"
					title="Jump to live"
					on:click|preventDefault={() => player?.toRealtime()}
					disabled={state === PlayerState.Loading || state === PlayerState.Error}>LIVE</button
				>
				<button
					title="Volume"
					on:click|preventDefault={toggleMuted}
					disabled={state === PlayerState.Error}
				>
					<Volume {volume} />
				</button>
				{#if videoEl}
					<input
						class="volume"
						type="range"
						min="0"
						max="1"
						step="0.01"
						disabled={state === PlayerState.Error}
						bind:value={volume}
					/>
				{/if}
			</div>
			<div>
				{#if variants}
					<select bind:value={selectedVariant} disabled={state === PlayerState.Error}>
						<option value={-1}>{autoVariantName(selectedVariant, currentVariant)}</option>
						{#each variants as variant, index}
							<option value={index} selected={index === selectedVariant}
								>{friendlyVariantName(variant)}</option
							>
						{/each}
					</select>
				{/if}
				<button
					title="Clip"
					on:click|preventDefault={onClipClick}
					disabled={state === PlayerState.Loading || state === PlayerState.Error}
				>
					<Clip />
				</button>
				{#if showPip && document.pictureInPictureEnabled && videoEl?.requestPictureInPicture !== undefined}
					<button
						title={pip ? "Exit picture-in-picture" : "Enter picture-in-picture"}
						on:click|preventDefault={togglePictureInPicture}
					>
						{#if pip}
							<ExitPictureInPicture />
						{:else}
							<EnterPictureInPicture />
						{/if}
					</button>
				{/if}
				{#if showTheater}
					<button
						title={theaterMode ? "Exit theater mode" : "Enter theater mode"}
						on:click|preventDefault={toggleTheaterMode}
					>
						{#if theaterMode}
							<ExitTheaterMode />
						{:else}
							<EnterTheaterMode />
						{/if}
					</button>
				{/if}
				<button title="Enter fullscreen" on:click|preventDefault={toggleFullscreen}>
					{#if fullscreen}
						<FullscreenMinimize />
					{:else}
						<FullscreenMaximize />
					{/if}
				</button>
			</div>
		</div>
	{/if}
	{#if debugOverlay}
		<DebugOverlay
			{player}
			playerToken={live.playerToken ?? undefined}
			{videoEl}
			on:close={() => (debugOverlay = false)}
		/>
	{/if}
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.player {
		position: relative;
		/* For some reason I don't get, this needs to be flex. Otherwise the player div is too high. */
		display: flex;

		color: $textColor;

		&.theater-mode {
			height: 100%;
		}

		&:not(.theater-mode) {
			max-height: 85vh;
		}
	}

	video {
		background-color: black;
		aspect-ratio: 16 / 9;
		width: 100%;
		transition: filter 0.2s;

		&.darken {
			filter: brightness(0.8);
		}
	}

	.center-icons {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;

		display: flex;
		justify-content: center;
		align-items: center;
		flex-direction: column;
		gap: 1rem;

		pointer-events: none;

		opacity: 0;
		transition: opacity 0.2s;
		&.show {
			opacity: 1;
		}
	}

	button,
	select {
		color: $textColor;
		border-radius: 0.5rem;
		font-size: 0.75rem;
		font-weight: 600;
		background-color: transparent;
		border: none;

		display: flex;
		align-items: center;
		padding: 0.25rem;

		&:hover:not(:disabled),
		&:focus-visible:not(:disabled) {
			background-color: rgba($bgColorLight, 0.8);
		}

		&:disabled {
			opacity: 0.5;
			cursor: default;
		}
	}

	.player:not(:hover):not(:focus-visible) {
		.controls {
			opacity: 0;
			pointer-events: none;
		}
	}

	.player.controls-hidden {
		cursor: none;

		.controls {
			opacity: 0;
			pointer-events: none;
		}
	}

	.controls {
		transition: opacity 0.2s;

		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		padding: 0.5rem 0.5rem;
		background: linear-gradient(transparent 0%, rgba($bgColor, 0.5) 100%);

		display: flex;
		justify-content: space-between;
		align-items: center;

		& > div {
			display: flex;
			align-items: center;
			gap: 0.5rem;
		}

		.live {
			background-color: none;
			padding: 0.1rem 0.25rem;

			&:hover:not(:disabled),
			&:focus-visible:not(:disabled) {
				background-color: $liveColor;
			}
		}
	}

	.volume {
		appearance: none;
		-webkit-appearance: none;
		width: 8rem;
		background: transparent;
	}

	.volume:focus {
		outline: none;
	}

	.volume::-moz-range-thumb {
		appearance: none;
		width: 1rem;
		height: 1rem;
		border-radius: 50%;
		background: white;
		cursor: pointer;
	}

	.volume::-ms-thumb {
		width: 1rem;
		height: 1rem;
		border-radius: 50%;
		background: white;
		cursor: pointer;
	}

	.volume::-webkit-slider-thumb {
		-webkit-appearance: none;
		margin-top: -0.25rem;
		width: 1rem;
		height: 1rem;
		border-radius: 50%;
		background: white;
		cursor: pointer;
	}

	.volume::-webkit-slider-runnable-track {
		width: 100%;
		height: 0.5rem;
		cursor: pointer;
		background: rgba($textColor, 0.25);
		border-radius: 0.25rem;
	}

	.volume::-moz-range-track {
		width: 100%;
		height: 0.5rem;
		cursor: pointer;
		background: rgba($textColor, 0.25);
		border-radius: 0.25rem;
	}
</style>
