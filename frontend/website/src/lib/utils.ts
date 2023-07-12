export function viewersToString(viewers: number | null) {
	if (viewers === null) {
		return "Offline";
	}

	if (viewers < 1000) {
		return viewers.toString();
	} else if (viewers < 100000) {
		return (viewers / 1000).toFixed(1).replace(".", ",") + "K";
	} else if (viewers < 1000000) {
		return (viewers / 1000).toFixed(0) + "K";
	} else {
		return (viewers / 1000000).toFixed(1).replace(".", ",") + "M";
	}
}
