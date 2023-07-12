// We want to convert number of viewers to a string with commas.
// For example, 1000 should become 1K.
// 1300 should become 1,3K.
// Anything bigger than 100 000 should become 100K without any decimals.
// 1 000 000 should become 1M.
// 1 300 000 should become 1,3M.
export function viewersToString(viewers: number | null, labelled: boolean = false) {
	if (viewers === null) {
		return "Offline";
	}

	const count = new Intl.NumberFormat("en-US", {
		notation: "compact",
		unitDisplay: "narrow",
	}).format(viewers);

	if (labelled) {
		if (viewers === 1) {
			return `${count} viewer`;
		} else {
			return `${count} viewers`;
		}
	} else {
		return count;
	}
}
