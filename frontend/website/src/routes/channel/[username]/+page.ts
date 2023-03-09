export interface LoadParams {
	username: string;
	chatId: number;
}

export const load = ({ params }: { params: LoadParams }): { username: string; chatId: number } => {
	return {
		username: params.username,
		// we will need to query chatid for channel here
		chatId: parseInt(params.username),
	};
};
