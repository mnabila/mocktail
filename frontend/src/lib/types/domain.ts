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
    allowOrigins: '*' | string[];
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
    contentType?: string | null;
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
  username: string;
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

export type WorkspaceRole = 'owner' | 'editor' | 'viewer';
export type Role = WorkspaceRole;
export type CollectionSource = 'manual' | 'openapi';

// Response of GET /__admin/me: the signed-in user, their memberships, linked
// SSO identities, and the CSRF token for mutations.
export interface MeResponse {
  user: User;
  memberships: Membership[];
  providers: Identity[];
  csrf: string;
}

// A workspace member as returned by the members admin API (joins user + role).
export interface WorkspaceMember {
  id: string;
  userId: string;
  displayName: string;
  email: string;
  role: WorkspaceRole;
}

// Folders + mocks of one collection, loaded lazily for the sidebar tree.
export interface CollectionTree {
  folders: Folder[];
  mocks: Mock[];
}

export interface LogPageResponse {
  items: RequestLog[];
  nextCursor?: string | null;
}

// A select option: a bare value (its own label) or an explicit value+label.
export type SelectOption = string | number | boolean | null | { value: unknown; label?: string };

export type ToastType = 'success' | 'error';

export interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
}

export interface MockHeader {
  name: string;
  value: string;
}

export interface RateLimitConfig {
  limit: number;
  windowSecs: number;
  status: number;
}

// The in-progress mock being edited in the editor (flatter than the stored
// Mock: headers as {name,value}, content type split out, rate limit always set).
export interface MockEditorDraft {
  id?: string;
  method: string;
  pathTemplate: string;
  folderId: string | null;
  response: {
    status: number;
    headers: MockHeader[];
    body: string;
    contentType: string;
    customType: boolean;
  };
  priority: number;
  delayMs: number;
  fault: 'connectionReset' | 'malformedResponse' | null;
  rateLimit: RateLimitConfig;
}

export type ImportAction = 'added' | 'changed' | 'unchanged' | 'removed';

export interface ImportPreviewItem {
  key: string;
  method: string;
  pathTemplate: string;
  status: number;
  action: ImportAction;
  selected: boolean;
  tag?: string | null;
  summary?: string | null;
  operationId?: string | null;
  source?: string | null;
}

export interface ImportPreviewFolder {
  path: string;
  action: ImportAction;
}

export interface ImportPreviewResponse {
  items: ImportPreviewItem[];
  folders: ImportPreviewFolder[];
  spec?: { title?: string | null; version?: string | null; basePath?: string | null };
  summary: {
    added: number;
    changed: number;
    unchanged: number;
    removed: number;
    foldersAdded: number;
  };
}

export interface ImportCommitResponse {
  created: number;
  updated: number;
  deleted: number;
  foldersCreated: number;
}
