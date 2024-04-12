import { z } from "zod";
import { FieldStatusType, type FieldStatus } from "$/components/form/field.svelte";
import { browser } from "$app/environment";

export function isMobile() {
	return browser && window.innerWidth < 768;
}

export function isHover() {
	return browser && window.matchMedia("(hover: hover)").matches;
}

export function fieldsValid(status: (FieldStatus | undefined)[]) {
	for (const s of status) {
		if (!s || s.type !== FieldStatusType.Success) {
			return false;
		}
	}
	return true;
}

export async function passwordValidate(v: string): Promise<FieldStatus> {
	const valid = z
		.string()
		.min(8, "At least 8 characters")
		.max(100, "Maximum of 100 characters")
		.regex(/.*[A-Z].*/, "At least one uppercase character")
		.regex(/.*[a-z].*/, "At least one lowercase character")
		.regex(/.*\d.*/, "At least one number")
		.regex(/.*[`~<>?,./!@#$%^&*()\-_+="'|{}[\];:].*/, "At least one special character")
		.safeParse(v);

	if (!valid.success) {
		return { type: FieldStatusType.Error, message: valid.error.issues[0].message };
	}

	return { type: FieldStatusType.Success };
}

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

export function mouseTrap(el: HTMLElement, cb: (e: MouseEvent) => void) {
	const state = {
		cb,
		el,
		willTrigger: false,
		ready: false,
	};

	setTimeout(() => {
		state.ready = true;
	}, 10);

	const onMouseDown = (e: MouseEvent) => {
		state.willTrigger = state.ready && e.button === 0 && !state.el.contains(e.target as Node);
	};

	window.addEventListener("mousedown", onMouseDown);

	const onMouseUp = (e: MouseEvent) => {
		state.willTrigger = state.willTrigger && e.button === 0 && !state.el.contains(e.target as Node);
		if (state.willTrigger) {
			state.cb(e);
		}
	};

	window.addEventListener("mouseup", onMouseUp);

	return {
		update(cb: (e: MouseEvent) => void) {
			state.cb = cb;
		},
		destroy() {
			window.removeEventListener("mousedown", onMouseDown);
			window.removeEventListener("mouseup", onMouseUp);
		},
	};
}
