<script lang="ts">
	import { viewersToString } from "$lib/utils";
	import Tag from "$components/tag.svelte";

	export let streamer: string;
	export let title: string;
	export let tags: string[];
	export let viewers: number;
	export let avatar: string;
	export let preview: string;
	export let special: boolean = false;

	$: viewersString = viewersToString(viewers);
</script>

<div class="stream-preview" class:special>
	<div class="preview">
		<img src={preview} alt="stream preview" class="preview-image" />
		<div class="viewers-bar" />
	</div>
	<span class="viewers-text">{viewersString} viewers</span>
	<img src={avatar} alt="streamer avatar" class="avatar" />
	<span class="title">{title}</span>
	<span class="streamer">{streamer}</span>
	<div class="tags">
		{#each tags as tag}
			<Tag content={tag} />
		{/each}
	</div>
</div>

<style lang="scss">
	$viewerBarWidth: 0.15rem;

	.stream-preview {
		display: grid;
		grid-template:
			"preview preview"
			"avatar title"
			"avatar streamer"
			"tags tags";
		grid-template-rows: auto auto auto auto;
		grid-template-columns: auto 1fr;

		position: relative;

		width: 20rem;

		font-family: "Inter", sans-serif;

		&.special {
			.viewers-bar,
			.avatar {
				display: none;
			}

			grid-template:
				"streamer streamer"
				"preview title"
				"preview title"
				"preview tags"
				"preview viewers";

			grid-template-rows: auto auto auto auto auto;
			grid-template-columns: auto 1fr;

			width: 41.5rem;

			background: radial-gradient(
				69.84% 128.99% at 0% 0%,
				#ffabab 0%,
				#fe28a1 50%,
				rgba(33, 0, 21, 0) 100%
			);
			padding: 0 1rem;

			.streamer {
				font-size: 8rem;
				color: white;
				font-weight: 500;
				text-transform: lowercase;
				letter-spacing: -0.7rem;
				line-height: 4rem;
				margin-bottom: -1rem;
				position: relative;
				z-index: 1;
			}

			.title {
				font-size: 1rem;
				font-weight: 500;
				color: white;
				align-self: self-end;
				white-space: pre-wrap;
				padding: 3.25rem 0 0 0;
			}

			.preview-image {
				width: 22.75rem;
				height: 12.75rem;
			}

			column-gap: 1rem;
			row-gap: 1rem;

			.viewers-text {
				grid-area: viewers;
				border-top-right-radius: 0;
				border-bottom-right-radius: 0;
				background-color: transparent;
				border-left: none;
				font-family: "IBM Plex Mono", monospace;
				color: #a0a0a0;
			}

			.viewers-text::before {
				content: "";
				display: inline-block;
				width: 0.4rem;
				height: 0.4rem;
				background-color: #e91916;
				border-radius: 50%;
				margin-right: 0.4rem;
				margin-bottom: 0.1rem;
			}
		}
	}

	.preview {
		grid-area: preview;
		position: relative;
		display: flex;
		height: min-content;

		&:hover {
			// When we hover we want to make the viewers bar the same size as the .stream-preview
			.viewers-bar {
				top: 0;
				height: 100%;
			}
		}
	}

	.preview-image {
		align-self: flex-end;
		width: 19.75rem;
		height: 11.125rem;
		object-fit: cover;
	}

	.avatar {
		grid-area: avatar;
		width: 4rem;
		height: 4rem;
		border-radius: 50%;
		padding: 0.5rem;
	}

	.title {
		grid-area: title;
		font-size: 1rem;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		align-self: self-end;
		margin-bottom: 0.25rem;
	}

	.streamer {
		grid-area: streamer;
		font-size: 0.875rem;
		font-weight: 400;
		color: #a0a0a0;
	}

	.viewers-text,
	.viewers-bar {
		position: absolute;
		top: 0.25rem;
		padding: 0.25rem 0.375rem;
	}

	.viewers-text {
		background-color: rgba(0, 0, 0, 0.3);
		font-weight: 500;
		text-align: right;
		display: flex;
		border-left: $viewerBarWidth solid #f00;
		left: -$viewerBarWidth;
		align-items: center;
		border-top-right-radius: 0.25rem;
		border-bottom-right-radius: 0.25rem;
		user-select: none;
	}

	.viewers-bar {
		border-left: $viewerBarWidth solid #f00;
		left: -$viewerBarWidth;
		position: absolute;
		height: 0;
		transition: height 0.2s ease-in-out, top 0.2s ease-in-out;
		color: transparent;
		user-select: none;
	}

	.tags {
		grid-area: tags;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
</style>
