<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import init, { Player } from "@scuffle/player";
	import type { EventError, Variant } from "@scuffle/player";
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

	export let streamId: string;
	export let controls = true;
	export let showPip = true;
	export let showTheater = true;
	export let muted = false;

	const _streamUrl = `https://troy-edge.scuffle.tv/${streamId}/master.m3u8`;

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
	let manifest: Variant[];
	let currentVariantId: number;

	// This is only used for hiding the controls when the mouse is not moving anymore
	let controlsHidden = false;
	let controlsHiddenTimeout: NodeJS.Timeout | number;

	let theaterMode = false;
	let pip = false;
	let fullscreen = false;
	let audioOnly = false;
	let selectedVariant: number;

	$: {
		topNavHidden.set(theaterMode);
		sideNavHidden.set(theaterMode);
	}

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
		console.log(player.variants);
		manifest = player.variants;
	}

	function onVariantChange() {
		let variant = manifest.at(player.variantId);
		if (variant) {
			currentVariantId = player.variantId;
			console.log(`Switched to ${variant.video_track?.name ?? "audio only"}`);
			audioOnly = !variant.video_track;
		} else {
			console.error("switched to unkonwn variant");
		}
	}

	function onError(evt: EventError) {
		state = PlayerState.Error;
		console.log(`⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠿⠛⠋⠉⣉⣉⠙⠿⠋⣠⢴⣊⣙⢿⣿⣿
⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠟⠋⠁⠀⢀⠔⡩⠔⠒⠛⠧⣾⠊⢁⣀⣀⣀⡙⣿
⣿⣿⣿⣿⣿⣿⣿⠟⠛⠁⠀⠀⠀⠀⠀⡡⠊⠀⠀⣀⣠⣤⣌⣾⣿⠏⠀⡈⢿⡜
⣿⣿⣿⠿⠛⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠡⣤⣶⠏⢁⠈⢻⡏⠙⠛⠀⣀⣁⣤⢢
⣿⠋⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠰⣄⡀⠣⣌⡙⠀⣘⡁⠜⠈⠑⢮⡭⠴⠚⠉⠀
⠁⠀⢀⠔⠁⣀⣤⣤⣤⣤⣤⣄⣀⠀⠉⠉⠉⠉⠉⠁⠀⠀⠀⠀⠀⠁⠀⢀⣠⢠
⡀⠀⢸⠀⢼⣿⣿⣶⣭⣭⣭⣟⣛⣛⡿⠷⠶⠶⢶⣶⣤⣤⣤⣶⣶⣾⡿⠿⣫⣾
⠇⠀⠀⠀⠈⠉⠉⠉⠉⠉⠙⠛⠛⠻⠿⠿⠿⠷⣶⣶⣶⣶⣶⣶⣶⣶⡾⢗⣿⣿
⣦⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣠⣴⣿⣶⣾⣿⣿⣿
⣿⣿⣿⣷⣶⣤⣄⣀⣀⣀⡀⠀⠀⠀⠀⠀⠀⢀⣀⣤⣝⡻⣿⣿⣿⣿⣿⣿⣿⣿
⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦⡹⣿⣿⣿⣿⣿⣿ Player Error`);
		console.error(evt);
	}

	function onShutdown() {
		console.log("shutdown");
		videoEl.pause();
	}

	onMount(() => {
		init().then(() => {
			player = new Player(videoEl, {
				organization_id: "...",
			});

			player.on("manifestloaded", onManifestLoaded);
			player.on("variant", onVariantChange);
			player.on("error", onError);
			player.on("destroyed", onShutdown);

			videoEl.play();
		});
	});

	onDestroy(() => {
		if (player) {
			player.destroy();
		}
	});

	function onPlayClick() {
		switch (state) {
			case PlayerState.Playing:
				videoEl.pause();
				break;
			case PlayerState.Loading:
			case PlayerState.Paused:
				videoEl.play();
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

	function jumpToLive() {
		videoEl.play();
		if (!videoEl.buffered.length) return;

		if (player.lowLatency) {
			videoEl.currentTime = videoEl.buffered.end(videoEl.buffered.length - 1) - 0.5;
		} else {
			videoEl.currentTime = videoEl.buffered.end(videoEl.buffered.length - 1) - 2;
		}
	}

	function toggleMuted() {
		videoEl.muted = !videoEl.muted;
	}

	function togglePictureInPicture() {
		if (pip) {
			document.exitPictureInPicture().then(() => (pip = false));
		} else {
			videoEl.requestPictureInPicture().then(() => (pip = true));
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
		if ($authDialog) return;
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
>
	<video
		bind:this={videoEl}
		on:playing={() => (state = PlayerState.Playing)}
		on:play={() => (state = PlayerState.Playing)}
		on:pause={() => (state = PlayerState.Paused)}
		on:waiting={() => (state = PlayerState.Loading)}
		preload="metadata"
		autoplay
		class:paused={state === PlayerState.Paused}
		class:audio-only={audioOnly}
		{muted}
	>
		<!-- No captions, this must be specified explicitly to suppress an a11y warning -->
		<track kind="captions" />
		<span>Sorry, your browser can't play this</span>
	</video>
	<div class="center-icons">
		{#if state === PlayerState.Error}
			<Lightning />
			<span>Something went wrong</span>
		{:else if state === PlayerState.Loading}
			<Spinner />
		{:else if state === PlayerState.Paused}
			<Pause size={48} />
		{:else if audioOnly}
			<Volume size={48} />
			<span>Audio Only</span>
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
					on:click|preventDefault={jumpToLive}
					disabled={state === PlayerState.Loading || state === PlayerState.Error}>LIVE</button
				>
				<button
					title="Volume"
					on:click|preventDefault={toggleMuted}
					disabled={state === PlayerState.Error}
				>
					<Volume muted={videoEl?.muted} />
				</button>
			</div>
			<div>
				{#if manifest}
					<select bind:value={selectedVariant}>
						<option value={-1}>
							auto
							{selectedVariant === -1
								? ` (${manifest.at(currentVariantId)?.video_track?.name ?? "audio-only"})`
								: ""}
						</option>
						{#each manifest as variant, index}
							<option value={index} selected={index === selectedVariant}
								>{variant.video_track?.name ?? "audio-only"}</option
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
				{#if showPip && videoEl?.requestPictureInPicture !== undefined}
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
</div>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.player {
		grid-area: player;

		position: relative;
		/* For some reason I don't get, this needs to be flex. Otherwise the player div is too high. */
		display: flex;

		color: $textColor;

		&.theater-mode {
			height: 100%;
		}
		&:not(.theater-mode) {
			max-height: calc(100vh - $topNavHeight - 5.75rem);
		}
	}

	video {
		background-color: black;
		aspect-ratio: 16 / 9;
		width: 100%;

		&.paused {
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
	}

	button,
	select {
		color: $textColor;
		border-radius: 0.5rem;
		font-size: 0.75rem;
		font-weight: 600;

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
</style>
