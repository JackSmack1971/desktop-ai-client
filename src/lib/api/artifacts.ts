import { invoke } from '@tauri-apps/api/core';
import type { ArtifactContentType } from '$lib/api/chat';

export type ArtifactPreviewResponse = {
	artifact_id: string;
	content_type: ArtifactContentType;
	srcdoc: string;
};

export async function artifactGet(
	artifactId: string,
): Promise<ArtifactPreviewResponse> {
	return invoke<ArtifactPreviewResponse>('artifact_get', { artifactId });
}

export async function artifactDismiss(artifactId: string): Promise<void> {
	await invoke('artifact_dismiss', { artifactId });
}
