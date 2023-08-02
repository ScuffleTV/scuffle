<script lang="ts">
	export let content: string;
</script>

<span class="tag">{content}</span>

<style lang="scss">
	@import "../assets/styles/variables.scss";

	.tag {
		padding: 0.2rem 0.5rem;
		font-size: 0.85rem;
		font-weight: 500;
		border: 1px dashed #383838;
		color: $textColorLight;
		transition: color 0.1s ease-in-out, background-color 0.5s ease-in-out,
			border 0.5s ease-in-out;
		user-select: none;

		&:hover {
			color: $textColor;
			border-style: solid;
		}
		&:active {
			background-color: #2fc0f465;
			border-color: #52e2f2c4;
			transition: unset;
		}
	}
</style>
