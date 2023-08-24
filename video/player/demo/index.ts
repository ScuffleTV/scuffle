import init, { Player } from "../pkg/video_player";

await init();

const player = new Player();

console.log(player);

const video = document.getElementById("video") as HTMLVideoElement;
const bufferSize = document.getElementById("buffer-size") as HTMLElement;
const videoTime = document.getElementById("video-time") as HTMLElement;
const frameRate = document.getElementById("frame-rate") as HTMLElement;
const resolution = document.getElementById("resolution") as HTMLElement;
const droppedFrames = document.getElementById("dropped-frames") as HTMLElement;
const bandwidth = document.getElementById("bandwidth") as HTMLElement;
const variantId = document.getElementById("variant-id") as HTMLElement;

const selectTracksDiv = document.getElementById("select-tracks") as HTMLDivElement;
const forceTracksDiv = document.getElementById("force-tracks") as HTMLDivElement;

const toggleLowLatency = document.getElementById("toggle-low-latency") as HTMLButtonElement;
const toggleAbr = document.getElementById("toggle-abr") as HTMLButtonElement;
const jumpToLive = document.getElementById("jump-to-live") as HTMLButtonElement;

let lastFrameTime = 0;
let frameCount = 0;

player.lowLatency = true;
player.abrEnabled = true;

toggleAbr.innerText = player.abrEnabled ? "Disable ABR" : "Enable ABR";
toggleLowLatency.innerText = player.lowLatency ? "Disable Low Latency" : "Enable Low Latency";

toggleLowLatency.addEventListener("click", () => {
	player.lowLatency = !player.lowLatency;
	toggleLowLatency.innerText = player.lowLatency ? "Disable Low Latency" : "Enable Low Latency";
});

toggleAbr.addEventListener("click", () => {
	player.abrEnabled = !player.abrEnabled;
	toggleAbr.innerText = player.abrEnabled ? "Disable ABR" : "Enable ABR";
});

jumpToLive.addEventListener("click", () => {
	if (!video.buffered.length) return;

	if (player.lowLatency) {
		video.currentTime = video.buffered.end(video.buffered.length - 1) - 0.5;
	} else {
		video.currentTime = video.buffered.end(video.buffered.length - 1) - 2;
	}
});

const loop = () => {
	if (video.buffered.length) {
		bufferSize.innerText = `${(
			video.buffered.end(video.buffered.length - 1) - video.currentTime
		).toFixed(3)}`;
	} else {
		bufferSize.innerText = "0";
	}

	videoTime.innerText = `${video.currentTime.toFixed(3)}`;
	resolution.innerText = `${video.videoWidth}x${video.videoHeight}`;

	const quality = video.getVideoPlaybackQuality();

	droppedFrames.innerText = `${quality.droppedVideoFrames}`;

	const now = performance.now();
	if (now - lastFrameTime >= 1000) {
		frameRate.innerText = `${quality.totalVideoFrames - frameCount}`;
		frameCount = quality.totalVideoFrames;
		lastFrameTime = now;
	}

	window.requestAnimationFrame(loop);
};

window.requestAnimationFrame(loop);

player.on("error", (evt) => {
	console.error(evt);
});

player.on("abrchange", (evt) => {
	const bdw = evt.bandwidth || 0;

	// Convert bps to either kbps or mbps depending on the size
	if (bdw > 1024 * 1024 * 8) {
		bandwidth.innerText = `${(bdw / 1024 / 1024).toFixed(2)}mbps`;
	} else {
		bandwidth.innerText = `${(bdw / 1024).toFixed(2)}kbps`;
	}

	toggleAbr.innerText = evt.enabled ? "Disable ABR" : "Enable ABR";
});

player.on("variantchange", (evt) => {
	variantId.innerText = `${evt.variant_id}`;
});

player.on("manifestloaded", (evt) => {
	selectTracksDiv.innerHTML = "";
	forceTracksDiv.innerHTML = "";

	evt.variants.forEach((variant) => {
		const button = document.createElement("button");
		button.innerText = `${variant.group} - ${variant.name}`;
		button.addEventListener("click", () => {
			player.nextVariantId = variant.id;
		});
		selectTracksDiv.appendChild(button);

		const forceButton = document.createElement("button");
		forceButton.innerText = `${variant.group} - ${variant.name}`;
		forceButton.addEventListener("click", () => {
			player.variantId = variant.id;
		});
		forceTracksDiv.appendChild(forceButton);
	});
});

player.on("shutdown", () => {
	video.pause();
	video.src = "";
	video.load();
});

const loadButton = document.getElementById("load") as HTMLButtonElement;
const inputUrl = document.getElementById("input-url") as HTMLInputElement;

loadButton.addEventListener("click", () => {
	player.load(inputUrl.value);
	player.attach(video);

	// Update URL fragment
	const url = new URL(window.location.href);
	url.hash = inputUrl.value;
	window.history.replaceState({}, "", url.href);
});

// Get URL fragment for the predefined url
const url = new URL(window.location.href);

const urlFragment = url.hash.slice(1);

if (urlFragment) {
	inputUrl.value = urlFragment;
	loadButton.click();
}
