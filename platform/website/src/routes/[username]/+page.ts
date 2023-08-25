import { redirect } from "@sveltejs/kit";
import type { PageLoadEvent } from "./$types";

export async function load({ parent }: PageLoadEvent) {
	const data = await parent();

	// When offline
	if (!data.user.channel.liveViewerCount && data.user.channel.liveViewerCount !== 0) {
		throw redirect(307, `/${data.user.username}/home`);
	}

	return { ...data };
}
