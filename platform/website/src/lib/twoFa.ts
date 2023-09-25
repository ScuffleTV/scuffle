function content(codes: string[]) {
	const jointCodes = codes.join("\n");
	const now = new Date();
	return `This file contains the 2-Factor-Authentication backup codes for your Scuffle account.
Please keep this file in a secure place or print it. Store these codes as long as you have 2FA enabled on your account.
Downloaded at ${now.toLocaleDateString()} ${now.toLocaleTimeString()}.

${jointCodes}
`;
}

export function downloadBackupCodes(backupCodes?: string[]) {
	if (backupCodes) {
		const data = content(backupCodes);
		const blob = new Blob([data], { type: "text/plain" });
		const url = URL.createObjectURL(blob);
		const link = document.createElement("a");
		link.style.display = "none";
		link.href = url;
		link.download = "scuffle-backup-codes.txt";
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
		URL.revokeObjectURL(url);
	}
}
