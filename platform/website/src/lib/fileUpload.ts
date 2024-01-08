import { sessionToken } from "$/store/auth";
import { get } from "svelte/store";

export async function uploadFile(url: string, metadata: object, data: Blob, captcha: string) {
    const formData = new FormData();
    formData.append("metadata", JSON.stringify(metadata));
    formData.append("file", data);
    formData.append("captcha", captcha);

    return await fetch(url, {
        method: 'POST',
        headers: {
            Authorization: `Bearer ${get(sessionToken)}`,
        },
        body: formData,
    });
}
