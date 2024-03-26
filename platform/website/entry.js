import { server as app } from "./build/index.js";

let shutdownCalled = false;

const shutdown = () => {
	console.log("Shutting down...");
	if (shutdownCalled) {
		console.log("Shutdown already called, forcing exit...");
		process.exit(1);
	}

	shutdownCalled = true;

	app.server.closeAllConnections();
	app.server.close();
};

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
