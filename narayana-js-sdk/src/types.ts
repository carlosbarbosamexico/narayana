// Type definitions for NarayanaDB SDK

export type DataType =
  | 'int8'
  | 'int16'
  | 'int32'
  | 'int64'
  | 'uint8'
  | 'uint16'
  | 'uint32'
  | 'uint64'
  | 'float32'
  | 'float64'
  | 'boolean'
  | 'string'
  | 'binary'
  | 'timestamp'
  | 'date'
  | 'json';

export interface Field {
  name: string;
  dataType: DataType;
  nullable?: boolean;
  defaultValue?: any;
}

export interface Schema {
  fields: Field[];
}

export interface Table {
  id: string;
  name: string;
  schema: Schema;
  createdAt: number;
  rowCount: number;
}

export interface Database {
  id: string;
  name: string;
  createdAt: number;
  tableCount: number;
  permissions?: DatabasePermissions;
}

export interface DatabasePermissions {
  read: boolean;
  write: boolean;
  create: boolean;
  delete: boolean;
  admin: boolean;
}

export interface QueryOptions {
  limit?: number;
  offset?: number;
  columns?: string[];
  where?: WhereClause;
  orderBy?: OrderBy[];
  groupBy?: string[];
  having?: WhereClause;
  join?: Join[];
}

export interface WhereClause {
  column: string;
  operator: '=' | '!=' | '>' | '<' | '>=' | '<=' | 'IN' | 'NOT IN' | 'LIKE' | 'ILIKE' | 'BETWEEN' | 'IS NULL' | 'IS NOT NULL';
  value?: any;
  and?: WhereClause[];
  or?: WhereClause[];
}

export interface OrderBy {
  column: string;
  direction: 'ASC' | 'DESC';
}

export interface Join {
  table: string;
  on: { left: string; right: string };
  type?: 'INNER' | 'LEFT' | 'RIGHT' | 'FULL';
}

export interface QueryResult<T = any> {
  rows: T[];
  columns: string[];
  rowCount: number;
  executionTimeMs: number;
  metadata?: QueryMetadata;
}

export interface QueryMetadata {
  scannedRows: number;
  indexesUsed: string[];
  cached: boolean;
  optimized: boolean;
}

export interface InsertOptions {
  batch?: boolean;
  batchSize?: number;
  returnIds?: boolean;
}

export interface UpdateOptions {
  where: WhereClause;
  returnUpdated?: boolean;
}

export interface DeleteOptions {
  where: WhereClause;
  returnDeleted?: boolean;
}

export interface SearchOptions {
  query: string;
  fuzzy?: boolean;
  typoTolerance?: number;
  semantic?: boolean;
  synonyms?: boolean;
  filters?: WhereClause[];
  sort?: OrderBy[];
  limit?: number;
  offset?: number;
}

export interface SearchResult<T = any> {
  results: Array<{
    id: string;
    score: number;
    data: T;
    highlights?: Highlight[];
  }>;
  total: number;
  tookMs: number;
  suggestions?: string[];
  relatedQueries?: string[];
}

export interface Highlight {
  field: string;
  snippets: string[];
}

export interface RealtimeSubscription {
  unsubscribe: () => void;
  on: (event: 'data' | 'error' | 'close', callback: (data: any) => void) => void;
}

export interface StreamOptions {
  batchSize?: number;
  onProgress?: (progress: StreamProgress) => void;
}

export interface StreamProgress {
  rowsProcessed: number;
  totalRows: number;
  percentage: number;
  elapsedMs: number;
}

export interface BatchOperation {
  type: 'insert' | 'update' | 'delete' | 'query';
  table: string;
  data?: any;
  where?: WhereClause;
  query?: string;
}

export interface BatchResult {
  operations: Array<{
    index: number;
    success: boolean;
    result?: any;
    error?: string;
  }>;
  totalTimeMs: number;
}

export interface Transaction {
  commit: () => Promise<void>;
  rollback: () => Promise<void>;
  insert: (table: string, data: any) => Promise<void>;
  update: (table: string, data: any, where: WhereClause) => Promise<void>;
  delete: (table: string, where: WhereClause) => Promise<void>;
  query: <T = any>(query: string | QueryOptions) => Promise<QueryResult<T>>;
}

export interface WebhookConfig {
  id?: string;
  name: string;
  url: string;
  events: WebhookEvent[];
  scope: WebhookScope;
  secret?: string;
  enabled?: boolean;
}

export type WebhookEvent = 
  | 'insert'
  | 'update'
  | 'delete'
  | 'create'
  | 'drop'
  | 'query';

export type WebhookScope = 
  | { type: 'global' }
  | { type: 'database'; database: string }
  | { type: 'table'; database: string; table: string }
  | { type: 'column'; database: string; table: string; column: string };

export interface ClientConfig {
  url: string;
  apiKey?: string;
  token?: string;
  database?: string;
  timeout?: number;
  retries?: number;
  enableRealtime?: boolean;
  enableStreaming?: boolean;
  enableQueryLearning?: boolean;
  permissions?: {
    database: string;
    permissions: DatabasePermissions;
  }[];
  cache?: {
    ttl?: number;
    maxSize?: number;
    strategy?: 'lru' | 'lfu' | 'fifo';
  };
}

// Server-side specific types
export interface StreamSubscription {
  id: string;
  close?: () => void;
  on?: (event: string, handler: Function) => void;
}

export interface EventSubscription {
  id: string;
  stream: string;
  handler: (event: NativeEvent) => void;
  close?: () => void;
}

export interface NativeEvent {
  id?: string;
  stream: string;
  eventType: string;
  payload: any;
  timestamp?: number;
  headers?: Record<string, string>;
}

export interface QuantumSyncConfig {
  nodeId: string;
  peers: Array<{ nodeId: string; address: string }>;
  syncInterval?: number;
  enabled?: boolean;
}

export interface HumanSearchQuery {
  query: string;
  fuzzy?: boolean;
  typo_tolerance?: number;
  semantic?: boolean;
  synonyms?: boolean;
  limit?: number;
  offset?: number;
  filters?: any;
  sort?: any;
}

export interface QueryLearningConfig {
  enabled?: boolean;
  minExecutionCount?: number;
  optimizationThreshold?: number;
}

export interface MaterializedView {
  id: string;
  name: string;
  query: string;
  refreshStrategy: 'interval' | 'on_commit' | 'continuous' | 'on_demand';
  refreshInterval?: number;
  lastRefreshed?: number;
}

export interface PipelineOperation {
  type: string;
  operation: string;
  params?: any;
  dependsOn?: number[];
}

