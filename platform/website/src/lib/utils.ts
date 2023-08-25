export function viewersToString(viewers: number, labelled: boolean = false) {
	const count = formatBigNumber(viewers);

	if (!labelled) {
		return count;
	}

	if (viewers === 1) {
		return `${count} viewer`;
	} else {
		return `${count} viewers`;
	}
}

export function followersToString(followers: number) {
	const count = formatBigNumber(followers);

	if (followers === 1) {
		return `${count} follower`;
	} else {
		return `${count} followers`;
	}
}

function formatBigNumber(number: number): string {
	return new Intl.NumberFormat("en-US", {
		notation: "compact",
		unitDisplay: "narrow",
	}).format(number);
}

export function formatDuration(time: Date): string {
	const MILLIS_PER_MINUTE = 1000 * 60;
	const MILLIS_PER_HOUR = MILLIS_PER_MINUTE * 60;

	const duration = new Date().getTime() - time.getTime();

	const h = Math.floor(duration / MILLIS_PER_HOUR);
	let remainder = duration % MILLIS_PER_HOUR;

	const m = Math.floor(remainder / MILLIS_PER_MINUTE);
	remainder = duration % MILLIS_PER_MINUTE;

	const s = Math.floor(remainder / 1000);

	const HH = h ? `${formatSmallNumber(h)}:` : "";
	return `${HH}${formatSmallNumber(m)}:${formatSmallNumber(s)}`;
}

function formatSmallNumber(number: number): string {
	if (number < 10) {
		return `0${number}`;
	}
	return `${number}`;
}
