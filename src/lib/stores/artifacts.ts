import {
	artifactDismiss,
	artifactGet,
	type ArtifactPreviewResponse,
} from '$lib/api/artifacts';
import type { ArtifactContentType, ChatEvent } from '$lib/api/chat';
import { normalizeIpcError } from '$lib/api/errors';

type ArtifactState = 'idle' | 'loading' | 'ready' | 'dismissed' | 'error';

function createArtifactsStore() {
	let state = $state<ArtifactState>('idle');
	let artifactId = $state<string | null>(null);
	let contentType = $state<ArtifactContentType | null>(null);
	let preview = $state<string>('');
	let error = $state<string | null>(null);
	let requestNonce = 0;

	function applyPreview(response: ArtifactPreviewResponse): void {
		artifactId = response.artifact_id;
		contentType = response.content_type;
		preview = response.srcdoc;
		error = null;
		state = 'ready';
	}

	function receiveArtifact(
		event: Extract<ChatEvent, { type: 'ArtifactReady' }>,
		activeConversationId: string | null,
	): void {
		if (
			activeConversationId !== null &&
			event.conversation_id !== activeConversationId
		) {
			return;
		}
		artifactId = event.artifact_id;
		contentType = event.content_type;
		preview = event.preview;
		error = null;
		state = event.preview.trim() ? 'ready' : 'error';
		if (!event.preview.trim()) {
			error = 'Artifact could not be displayed safely.';
		}
	}

	async function reload(): Promise<void> {
		if (!artifactId) {
			state = 'idle';
			error = 'No artifact is available to reload.';
			return;
		}

		const nonce = ++requestNonce;
		state = 'loading';
		error = null;

		try {
			const response = await artifactGet(artifactId);
			if (nonce !== requestNonce) return;
			if (!response.srcdoc.trim()) {
				throw new Error('Artifact could not be displayed safely.');
			}
			applyPreview(response);
		} catch (e) {
			if (nonce !== requestNonce) return;
			error = normalizeIpcError(e);
			state = 'error';
			preview = '';
		}
	}

	async function dismiss(): Promise<void> {
		const currentId = artifactId;
		requestNonce += 1;
		state = 'dismissed';
		preview = '';
		error = null;

		if (currentId) {
			try {
				await artifactDismiss(currentId);
			} catch (e) {
				console.warn(
					'[artifacts-store] artifactDismiss rejected (best-effort):',
					e,
				);
			}
		}
	}

	return {
		get state() {
			return state;
		},
		get artifactId() {
			return artifactId;
		},
		get contentType() {
			return contentType;
		},
		get preview() {
			return preview;
		},
		get error() {
			return error;
		},
		get hasArtifact() {
			return artifactId !== null;
		},
		get isLoading() {
			return state === 'loading';
		},
		get isReady() {
			return state === 'ready' && preview.trim().length > 0;
		},
		get contentTypeLabel(): string {
			if (!contentType) return '';
			switch (contentType.type) {
				case 'Html':
					return 'HTML';
				case 'Svg':
					return 'SVG';
				case 'PlainText':
					return 'Text';
				case 'Code':
					return contentType.language.trim()
						? `Code · ${contentType.language}`
						: 'Code';
			}
			return '';
		},
		get statusMessage(): string {
			switch (state) {
				case 'idle':
					return 'No artifact yet';
				case 'loading':
					return 'Loading artifact…';
				case 'ready':
					return 'Artifact ready';
				case 'dismissed':
					return 'Artifact dismissed';
				case 'error':
					return error
						? `Could not load artifact. ${error}`
						: 'Could not load artifact.';
			}
			return 'Unknown artifact state';
		},
		receiveArtifact,
		reload,
		dismiss,
	};
}

export const artifactsStore = createArtifactsStore();
