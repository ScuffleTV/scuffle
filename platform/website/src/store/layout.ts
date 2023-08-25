import { browser } from "$app/environment";
import { writable } from "svelte/store";

function sideNavCollapsedInit(): boolean {
	if (browser && window.localStorage.getItem("layout_sideNavCollapsed") === "true") {
		return true;
	}
	return false;
}

export const sideNavCollapsed = writable(sideNavCollapsedInit());

export const sideNavHidden = writable(false);

export const topNavHidden = writable(false);

if (browser) {
	sideNavCollapsed.subscribe((collapsed) => {
		localStorage.setItem("layout_sideNavCollapsed", JSON.stringify(collapsed));
	});
}
