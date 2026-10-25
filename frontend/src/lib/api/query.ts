import { keepPreviousData, QueryClient, queryOptions } from '@tanstack/svelte-query';
import { api, loadMe } from './client';
import type {
  Collection,
  CollectionTree,
  Folder,
  ItemsResponse,
  LogPageResponse,
  MeResponse,
  Mock,
  RequestLog,
  Workspace,
  WorkspaceMember
} from '$lib/types/domain';

export interface LogQueryFilters {
  collectionId?: string | null;
  path?: string;
  status?: string;
  cursor?: string | null;
  limit?: number;
}

// Shared client for the portal's server state.
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: false
    }
  }
});

export const queryKeys = {
  me: () => ['me'] as const,
  workspaces: () => ['workspaces'] as const,
  members: (wsId?: string | null) => ['members', wsId] as const,
  collections: (wsId?: string | null) => ['collections', wsId] as const,
  collectionTree: (collectionId?: string | null) => ['collection-tree', collectionId] as const,
  mock: (mockId?: string | null) => ['mock', mockId] as const,
  logs: (wsId?: string | null, filters?: LogQueryFilters) => ['logs', wsId, filters ?? {}] as const,
  logDetail: (wsId?: string | null, requestId?: string | null) => ['log', wsId, requestId] as const
};

export function apiErrorMessage(err: unknown, fallback = 'failed'): string {
  if (
    err &&
    typeof err === 'object' &&
    'error' in err &&
    typeof (err as { error?: unknown }).error === 'string'
  ) {
    return (err as { error: string }).error;
  }
  if (err instanceof Error && err.message) {
    return err.message;
  }
  return fallback;
}

export function meQueryOptions(enabled = true) {
  return queryOptions({
    queryKey: queryKeys.me(),
    queryFn: (): Promise<MeResponse> => loadMe(),
    enabled
  });
}

export function workspacesQueryOptions(enabled = true) {
  return queryOptions({
    queryKey: queryKeys.workspaces(),
    queryFn: async (): Promise<Workspace[]> => {
      const res = await api.get<ItemsResponse<Workspace>>('/workspaces');
      return res.items;
    },
    enabled
  });
}

export function collectionsQueryOptions(wsId?: string | null) {
  return queryOptions({
    queryKey: queryKeys.collections(wsId),
    queryFn: async (): Promise<Collection[]> => {
      const res = await api.get<ItemsResponse<Collection>>(`/workspaces/${wsId!}/collections`);
      return res.items;
    },
    enabled: Boolean(wsId)
  });
}

export function membersQueryOptions(wsId?: string | null, opts?: { silent?: boolean }) {
  return queryOptions({
    queryKey: queryKeys.members(wsId),
    queryFn: async (): Promise<WorkspaceMember[]> => {
      const req = api.get<ItemsResponse<WorkspaceMember>>(`/workspaces/${wsId!}/members`);
      const res = opts?.silent
        ? await req.catch(() => ({ items: [] as WorkspaceMember[] }))
        : await req;
      return res.items;
    },
    enabled: Boolean(wsId)
  });
}

export function collectionTreeQueryOptions(collectionId?: string | null) {
  return queryOptions({
    queryKey: queryKeys.collectionTree(collectionId),
    queryFn: async (): Promise<CollectionTree> => {
      const [f, m] = await Promise.all([
        api
          .get<ItemsResponse<Folder>>(`/collections/${collectionId!}/folders`)
          .catch(() => ({ items: [] as Folder[] })),
        api
          .get<ItemsResponse<Mock>>(`/collections/${collectionId!}/mocks`)
          .catch(() => ({ items: [] as Mock[] }))
      ]);
      return { folders: f.items, mocks: m.items };
    },
    enabled: Boolean(collectionId)
  });
}

export function mockQueryOptions(mockId?: string | null, enabled = true) {
  return queryOptions({
    queryKey: queryKeys.mock(mockId),
    queryFn: (): Promise<Mock> => api.get<Mock>(`/mocks/${mockId!}`),
    enabled: Boolean(mockId) && enabled
  });
}

export function logsQueryOptions(wsId?: string | null, filters?: LogQueryFilters) {
  const limit = filters?.limit ?? 20;
  return queryOptions({
    queryKey: queryKeys.logs(wsId, filters),
    queryFn: (): Promise<LogPageResponse> => {
      const p = new URLSearchParams();
      if (filters?.collectionId) p.set('collectionId', filters.collectionId);
      p.set('limit', String(limit));
      if (filters?.path) p.set('path', filters.path);
      if (filters?.status) {
        if (/^\d{3}$/.test(filters.status)) p.set('status', filters.status);
        else p.set('statusClass', filters.status);
      }
      if (filters?.cursor) p.set('cursor', filters.cursor);
      return api.get<LogPageResponse>(`/workspaces/${wsId!}/logs?${p.toString()}`);
    },
    enabled: Boolean(wsId),
    placeholderData: keepPreviousData
  });
}

export function logDetailQueryOptions(wsId?: string | null, requestId?: string | null) {
  return queryOptions({
    queryKey: queryKeys.logDetail(wsId, requestId),
    queryFn: async (): Promise<RequestLog | null> => {
      const res = await api.get<ItemsResponse<RequestLog>>(
        `/workspaces/${wsId!}/logs?requestId=${encodeURIComponent(requestId!)}`
      );
      return res.items.find((l) => l.requestId === requestId) ?? res.items[0] ?? null;
    },
    enabled: Boolean(wsId && requestId)
  });
}
