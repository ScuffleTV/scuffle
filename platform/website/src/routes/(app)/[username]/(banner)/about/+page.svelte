<script lang="ts">
	import type { PageData } from "./$types";
	import { decodeTime } from "ulid";
	import moment from "moment";

	export let data: PageData;

	$: memberSince = moment.utc(decodeTime(data.user.id)).fromNow(true);
	$: lastLiveDuration =
		data.user.channel.lastLiveAt && moment.utc(data.user.channel.lastLiveAt).fromNow();
</script>

<pre>
- Last live {lastLiveDuration}
- Member since {memberSince}
</pre>
