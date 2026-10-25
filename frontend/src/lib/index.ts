// Barrel exports for shared UI primitives, API client, state, and domain types.
export { default as Icon } from './components/ui/Icon.svelte';
export { default as Modal } from './components/ui/Modal.svelte';
export { default as Select } from './components/ui/Select.svelte';
export { default as JsonBlock } from './components/ui/JsonBlock.svelte';
export { default as Toasts } from './components/ui/Toasts.svelte';

export { api, loadMe, setCsrf } from './api/client';
export {
  queryClient,
  queryKeys,
  apiErrorMessage,
  meQueryOptions,
  workspacesQueryOptions,
  collectionsQueryOptions,
  membersQueryOptions,
  collectionTreeQueryOptions,
  mockQueryOptions,
  logsQueryOptions,
  logDetailQueryOptions
} from './api/query';
export { uiState, wsIdFromSlug } from './state/session.svelte';
export { toast, dismiss, toasts } from './state/toast.svelte';
export * from './types/domain';
