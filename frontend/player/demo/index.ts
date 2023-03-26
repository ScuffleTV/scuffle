import { init, Player } from "../js/main";

await init();

const player = new Player();

console.log(player);

const video = document.getElementById("video") as HTMLVideoElement;
const bufferSize = document.getElementById("buffer-size") as HTMLElement;
const videoTime = document.getElementById("video-time") as HTMLElement;
const frameRate = document.getElementById("frame-rate") as HTMLElement;

const selectTrack0 = document.getElementById("select-track-0") as HTMLButtonElement;
const selectTrack1 = document.getElementById("select-track-1") as HTMLButtonElement;
const selectTrack2 = document.getElementById("select-track-2") as HTMLButtonElement;
const selectTrack3 = document.getElementById("select-track-3") as HTMLButtonElement;
const selectTrack4 = document.getElementById("select-track-4") as HTMLButtonElement;

const forceTrack0 = document.getElementById("force-track-0") as HTMLButtonElement;
const forceTrack1 = document.getElementById("force-track-1") as HTMLButtonElement;
const forceTrack2 = document.getElementById("force-track-2") as HTMLButtonElement;
const forceTrack3 = document.getElementById("force-track-3") as HTMLButtonElement;
const forceTrack4 = document.getElementById("force-track-4") as HTMLButtonElement;

const toggleLowLatency = document.getElementById("toggle-low-latency") as HTMLButtonElement;
const jumpToLive = document.getElementById("jump-to-live") as HTMLButtonElement;

let lastFrameTime = 0;
let frameCount = 0;

selectTrack0.addEventListener("click", () => {
	player.nextTrackId = 0;
});

selectTrack1.addEventListener("click", () => {
	player.nextTrackId = 1;
});

selectTrack2.addEventListener("click", () => {
	player.nextTrackId = 2;
});

selectTrack3.addEventListener("click", () => {
	player.nextTrackId = 3;
});

selectTrack4.addEventListener("click", () => {
	player.nextTrackId = 4;
});

forceTrack0.addEventListener("click", () => {
	player.forceTrackId = 0;
});

forceTrack1.addEventListener("click", () => {
	player.forceTrackId = 1;
});

forceTrack2.addEventListener("click", () => {
	player.forceTrackId = 2;
});

forceTrack3.addEventListener("click", () => {
	player.forceTrackId = 3;
});

forceTrack4.addEventListener("click", () => {
	player.forceTrackId = 4;
});

toggleLowLatency.addEventListener("click", () => {
	player.lowLatency = !player.lowLatency;
});

jumpToLive.addEventListener("click", () => {
	if (player.lowLatency) {
		video.currentTime = video.buffered.end(video.buffered.length - 1) - 0.5;
	} else {
		video.currentTime = video.buffered.end(video.buffered.length - 1) - 2;
	}
});

video.addEventListener("timeupdate", () => {
	bufferSize.innerText = video.buffered.end(video.buffered.length - 1) - video.currentTime + "";
	videoTime.innerText = video.currentTime + "";

	const quality = video.getVideoPlaybackQuality();

	const now = performance.now();
	if (now - lastFrameTime > 1000) {
		frameRate.innerText = quality.totalVideoFrames - frameCount + "";
		frameCount = quality.totalVideoFrames;
		lastFrameTime = now;
	}
});

player.lowLatency = false;

player.onerror = (evt) => {
	console.error(evt);
};

player.onmanifestloaded = (evt) => {
	console.log(evt);
};

player.load(
	// "http://192.168.2.177:9080/4f75cb30-6acf-4b1f-a91d-d9ae2c72c0cd/master.m3u8",
	"https://troy-edge.scuffle.tv/4f75cb30-6acf-4b1f-a91d-d9ae2c72c0cd/master.m3u8",
	// "http://192.168.2.177:9080/51636c0f-a2f1-46d6-9da1-07b386efff7a/03f92acb-fd92-4fb5-9023-5e27b82ba987/index.m3u8",
	// "http://192.168.2.177:9080/4def6aa7-6ae2-4a35-a473-d346de345e54/041c0b21-972d-4992-aca5-f010c01067c5/index.m3u8",
);

await player.attach(video);
