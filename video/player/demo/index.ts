import init, { Player } from "../pkg/video_player";

await init();

const video = document.getElementById("video") as HTMLVideoElement;

function bandwidthEstimate() {
	const bandwidthEstimate = localStorage.getItem("SCUFFLE_PLAYER_bandwidth-estimate");
	if (!bandwidthEstimate) {
		return undefined;
	}

	console.log("Using bandwidth estimate from local storage", bandwidthEstimate);

	return parseFloat(bandwidthEstimate);
}

function saveBandwidthEstimate(bandwidth: number) {
	localStorage.setItem(
		"SCUFFLE_PLAYER_bandwidth-estimate",
		`${Math.min(bandwidth, 16 * 1000 * 1000)}`,
	);
}

const player = new Player(video, {
	organization_id: "01HJES00BPBT13KS5BQM2V7EWS",
	server: "https://troy-edge.scuffle.tv",
	abr_default_bandwidth: bandwidthEstimate(),
	logging_level: "info",
});

declare global {
	interface Window {
		SCUFFLE_PLAYER: Player;
	}
}

window.SCUFFLE_PLAYER = player;

console.log(player);

const bufferSize = document.getElementById("buffer-size") as HTMLElement;
const videoTime = document.getElementById("video-time") as HTMLElement;
const videoDuration = document.getElementById("video-duration") as HTMLElement;
const latency = document.getElementById("latency") as HTMLElement;
const frameRate = document.getElementById("frame-rate") as HTMLElement;
const resolution = document.getElementById("resolution") as HTMLElement;
const droppedFrames = document.getElementById("dropped-frames") as HTMLElement;
const bandwidth = document.getElementById("bandwidth") as HTMLElement;
const variantId = document.getElementById("variant-id") as HTMLElement;
const realTime = document.getElementById("realtime") as HTMLElement;

const selectTracksDiv = document.getElementById("select-tracks") as HTMLDivElement;
const forceTracksDiv = document.getElementById("force-tracks") as HTMLDivElement;

const toggleLowLatency = document.getElementById("toggle-low-latency") as HTMLButtonElement;
const toggleAbr = document.getElementById("toggle-abr") as HTMLButtonElement;
const jumpToLive = document.getElementById("jump-to-live") as HTMLButtonElement;
const destroy = document.getElementById("destroy") as HTMLButtonElement;
const copyShareLink = document.getElementById("copy-share") as HTMLButtonElement;

copyShareLink.addEventListener("click", () => {
	const url = new URL(window.location.href);

	const params = new URLSearchParams();
	params.set("id", roomId.value);
	params.set("type", player.roomId ? "room" : "recording");
	params.set("currentTime", `${video.currentTime}`);

	url.hash = params.toString();

	navigator.clipboard.writeText(url.href);

	copyShareLink.innerText = "Copied!";
	copyShareLink.disabled = true;
	setTimeout(() => {
		copyShareLink.innerText = "Copy Share Link";
		copyShareLink.disabled = false;
	}, 1000);
});

let lastFrameTime = 0;
let frameCount = 0;

player.lowLatency = true;
player.abrEnabled = true;

toggleAbr.innerText = player.abrEnabled ? "Disable ABR" : "Enable ABR";
toggleLowLatency.innerText = player.lowLatency ? "Disable Low Latency" : "Enable Low Latency";

toggleLowLatency.addEventListener("click", () => {
	player.lowLatency = !player.lowLatency;
	jumpToLive.click();
	toggleLowLatency.innerText = player.lowLatency ? "Disable Low Latency" : "Enable Low Latency";
});

toggleAbr.addEventListener("click", () => {
	player.abrEnabled = !player.abrEnabled;
	toggleAbr.innerText = player.abrEnabled ? "Disable ABR" : "Enable ABR";
});

jumpToLive.addEventListener("click", () => {
	player.toRealtime();
});

destroy.addEventListener("click", () => {
	player.destroy();
});

let destroyed = false;

player.on("destroyed", () => {
	console.log("destroyed");
	destroyed = true;
});

const loop = () => {
	if (destroyed) return;

	try {
		if (video.buffered.length) {
			let found = false;
			for (let i = 0; i < video.buffered.length; i++) {
				const start = video.buffered.start(i);
				const end = video.buffered.end(i);
				if (video.currentTime >= start && video.currentTime <= end) {
					bufferSize.innerText = `${(end - video.currentTime).toFixed(3)}`;
					found = true;
					break;
				}
			}

			if (!found) {
				bufferSize.innerText = "-1";
			}
		} else {
			bufferSize.innerText = "0";
		}

		const currentTime = video.currentTime;
		const duration = video.seekable.length ? video.seekable.end(0) : 0;

		videoTime.innerText = `${currentTime.toFixed(3)}`;
		videoDuration.innerText = `${duration.toFixed(3)}`;
		latency.innerText = `${(duration - currentTime).toFixed(3)}`;

		resolution.innerText = `${video.videoWidth}x${video.videoHeight}`;

		saveBandwidthEstimate(player.bandwidth || 0);

		const bps = (player.bandwidth || 0) / 1000;

		if (bps > 3000) {
			bandwidth.innerText = `${(bps / 1000).toFixed(2)}Mbps`;
		} else {
			bandwidth.innerText = `${bps.toFixed(2)}Kbps`;
		}

		const quality = video.getVideoPlaybackQuality();

		droppedFrames.innerText = `${quality.droppedVideoFrames}`;

		const now = performance.now();
		if (now - lastFrameTime >= 1000) {
			frameRate.innerText = `${quality.totalVideoFrames - frameCount}`;
			frameCount = quality.totalVideoFrames;
			lastFrameTime = now;
		}
	} catch (e) {
		console.error(e);
	}

	window.requestAnimationFrame(loop);
};

window.requestAnimationFrame(loop);

player.on("error", (evt) => {
	console.error(evt);
});

player.on("abr", () => {
	toggleAbr.innerText = player.abrEnabled ? "Disable ABR" : "Enable ABR";
});

player.on("variant", (change) => {
	console.log(change);
	variantId.innerText = `${player.variantId}`;
});

player.on("manifestloaded", () => {
	selectTracksDiv.innerHTML = "";
	forceTracksDiv.innerHTML = "";

	player.variants.forEach((variant, idx) => {
		const button = document.createElement("button");
		button.innerText = `${variant.audio_track.name} - ${variant.video_track?.name}`;
		button.addEventListener("click", () => {
			player.nextVariantId = idx;
		});
		selectTracksDiv.appendChild(button);

		const forceButton = document.createElement("button");
		forceButton.innerText = `${variant.audio_track.name} - ${variant.video_track?.name}`;
		forceButton.addEventListener("click", () => {
			player.variantId = idx;
		});
		forceTracksDiv.appendChild(forceButton);
	});
});

player.on("started", () => {
	console.log("started");
});

player.on("stopped", () => {
	console.log("stopped");
});

player.on("finished", () => {
	console.log("finished");
});

player.on("realtime", () => {
	console.log("realtime mode changed", player.realtimeMode);
	realTime.innerText = `${player.realtimeMode}`;
});

player.on("visibility", () => {
	console.log("visibility changed", player.visible);
});

const loadRoomButton = document.getElementById("load-room") as HTMLButtonElement;
const loadRecordingButton = document.getElementById("load-recording") as HTMLButtonElement;
const roomId = document.getElementById("video-id") as HTMLInputElement;

function loadVideo(type: string, startTime: number) {
	if (type === "room") {
		player.loadRoom(roomId.value);
	} else if (type === "recording") {
		player.loadRecording(roomId.value);
	} else {
		throw new Error("Invalid type");
	}

	video.currentTime = startTime;

	// Update URL fragment
	const url = new URL(window.location.href);

	const params = new URLSearchParams();
	params.set("id", roomId.value);
	params.set("type", type);
	if (startTime != -1.0) {
		params.set("currentTime", `${startTime}`);
	}

	url.hash = params.toString();

	window.history.replaceState({}, "", url.href);

	let autoPlayed = false;
	const autoPlay = setInterval(() => {
		if (autoPlayed) return;
		autoPlayed = true;
		video
			.play()
			.then(() => {
				clearInterval(autoPlay);
			})
			.catch(() => {
				autoPlayed = false;
			});
	}, 100);
}

loadRoomButton.addEventListener("click", () => {
	loadVideo("room", -1.0);
});

loadRecordingButton.addEventListener("click", () => {
	loadVideo("recording", -1.0);
});

// Get URL fragment for the predefined url
const url = new URL(window.location.href);

const urlFragment = url.hash.slice(1);
if (urlFragment) {
	// Parse the fragment as query parameters
	const params = new URLSearchParams(urlFragment);

	const id = params.get("id");
	const type = params.get("type");
	const currentTime = params.get("currentTime") || "-1.0";
	if (id && type) {
		roomId.value = id;

		loadVideo(type, parseFloat(currentTime));
	}
}
