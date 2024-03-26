<div class="switch hide-on-desktop">
	<slot name="mobile" />
</div>
<div class="switch hide-on-mobile">
	<slot name="desktop" />
</div>

<style lang="scss">
	.switch {
		display: contents;
	}
</style>
