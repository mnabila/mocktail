// Shared UI & session state using Svelte 5 runes.

import type { MeResponse, Workspace } from '$lib/types/domain';

export interface ExpandTreeRequest {
  collectionId: string;
  ts: number;
}

export interface UiState {
  me: MeResponse | null;
  workspaces: Workspace[];
  searchQuery: string;
  newCollectionOpen: boolean;
  expandTreeRequest: ExpandTreeRequest | null;
}

export const uiState = $state<UiState>({
  me: null,
  workspaces: [],
  searchQuery: '',
  newCollectionOpen: false,
  expandTreeRequest: null
});

/** Resolve a workspace id from its slug (portal path segment). */
export function wsIdFromSlug(list: Workspace[], slug?: string): string | null {
  const ws = list.find((w) => w.slug === slug);
  return ws ? ws.id : null;
}
