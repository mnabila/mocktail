// Shared TypeScript domain interfaces for the mocktail portal.

export interface Workspace {
  id: string;
  name: string;
  slug: string;
  createdAt: number;
}

export interface Collection {
  id: string;
  workspaceId: string;
  name: string;
  slug: string;
  description: string;
  source: 'openapi' | 'manual';
  basePath: string;
  cors: {
    allowOriginsAny: boolean;
    allowOriginsList: string[];
    allowMethods: string[];
    allowHeaders: string[];
    exposeHeaders: string[];
    allowCredentials: boolean;
    maxAgeSecs: number;
  };
  version: number;
  createdAt: number;
}

export interface Folder {
  id: string;
  workspaceId: string;
  collectionId: string;
  name: string;
  parentId: string | null;
  createdAt: number;
}

export interface Mock {
  id: string;
  workspaceId: string;
  collectionId: string;
  folderId: string | null;
  method: string;
  pathTemplate: string;
  matchers: Array<{
    targetType: string;
    targetName: string | null;
    kindType: string;
    value: string | null;
    path: string | null;
    equals: string | null;
  }>;
  response: {
    status: number;
    headers: Array<[string, string]>;
    body: string;
  };
  priority: number;
  delayMs: number;
  fault: 'connectionReset' | 'malformedResponse' | null;
  rateLimit: {
    limit: number;
    windowSecs: number;
    status: number;
  } | null;
  createdAt: number;
  updatedAt: number;
}

export interface User {
  id: string;
  displayName: string;
  createdAt: number;
}

export interface Identity {
  provider: string;
  subject: string;
  email: string | null;
}

export interface Membership {
  id: string;
  userId: string;
  workspaceId: string;
  role: 'owner' | 'editor' | 'viewer';
}

export interface RequestLog {
  id: string;
  requestId: string;
  workspaceId: string;
  collectionId: string | null;
  receivedAt: number;
  method: string;
  path: string;
  requestHeaders: Array<[string, string]>;
  requestBody: string;
  matchedMockId: string | null;
  matchReason: string | null;
  status: number;
  response: {
    status: number;
    headers: Array<[string, string]>;
    body: string;
  };
  ip?: string;
  country?: string;
  lat?: number;
  lng?: number;
}

export interface LogQuery {
  coll?: string;
  path_contains?: string;
  status?: number;
  status_class?: number;
  cursor?: string;
  limit: number;
}

export interface ItemsResponse<T> {
  items: T[];
  nextCursor?: string;
}
