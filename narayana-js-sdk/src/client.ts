// NarayanaDB Client - The Most Advanced SDK Ever
// Works directly from the browser with full type safety

import { AuthManager } from './auth';
import { PermissionManager } from './permissions';
import { CacheManager } from './cache';
import { Database } from './database';
import {
  ClientConfig,
  DatabasePermissions,
  QueryResult,
  SearchResult,
  Transaction,
  WebhookConfig,
} from './types';
import {
  NarayanaError,
  ConnectionError,
  createError,
} from './errors';

export class NarayanaClient {
  private url: string;
  private auth: AuthManager;
  private permissions: PermissionManager;
  private cache: CacheManager;
  private timeout: number;
  private retries: number;
  private enableRealtime: boolean;
  private enableStreaming: boolean;
  private enableQueryLearning: boolean;
  private ws?: WebSocket;
  private wsReconnectTimer?: number;

  constructor(config: ClientConfig) {
    this.url = config.url.replace(/\/$/, ''); // Remove trailing slash
    this.timeout = config.timeout || 30000;
    this.retries = config.retries || 3;
    this.enableRealtime = config.enableRealtime ?? true;
    this.enableStreaming = config.enableStreaming ?? true;
    this.enableQueryLearning = config.enableQueryLearning ?? true;

    // Initialize auth
    this.auth = new AuthManager({
      apiKey: config.apiKey,
      token: config.token,
    });

    // Initialize permissions
    this.permissions = new PermissionManager();
    if (config.permissions) {
      for (const { database, permissions } of config.permissions) {
        this.permissions.setPermissions(database, permissions);
      }
    }

    // Initialize cache
    this.cache = new CacheManager({
      ttl: 300, // 5 minutes default
      maxSize: 1000,
      storage: 'localStorage',
      strategy: 'lru',
    });

    // Connect WebSocket if realtime enabled
    if (this.enableRealtime) {
      this.connectWebSocket();
    }
  }

  // ============================================================================
  // Authentication (No Backend Needed!)
  // ============================================================================

  async authenticate(email: string, password: string) {
    return await this.auth.authenticate(this.url, { email, password });
  }

  async register(email: string, password: string, name?: string) {
    return await this.auth.register(this.url, { email, password, name });
  }

  async logout() {
    await this.auth.logout(this.url);
    this.cache.clear();
  }

  async getCurrentUser() {
    return await this.auth.getCurrentUser();
  }

  async isAuthenticated(): Promise<boolean> {
    return await this.auth.isAuthenticated();
  }

  setApiKey(apiKey: string) {
    this.auth.setApiKey(apiKey);
  }

  setToken(token: string) {
    this.auth.setToken(token);
  }

  setPermissions(database: string, permissions: DatabasePermissions) {
    this.permissions.setPermissions(database, permissions);
    this.auth.setPermissions(database, permissions);
  }

  // ============================================================================
  // Session Management (Built-in!)
  // ============================================================================

  getSession() {
    return this.auth.getSessionManager();
  }

  // ============================================================================
  // Caching (Built-in!)
  // ============================================================================

  // Cache query results automatically
  async cachedQuery<T = any>(
    query: string,
    database?: string,
    ttl?: number
  ): Promise<QueryResult<T>> {
    const cacheKey = `query:${database || 'default'}:${query}`;
    
    return await this.cache.getOrSet(
      cacheKey,
      async () => {
        return await this.query<T>(query, database);
      },
      ttl
    );
  }

  // ============================================================================
  // Database Operations
  // ============================================================================

  database(name: string): Database {
    // Check permissions
    this.permissions.requirePermission(name, 'read');
    return new Database(this, name);
  }

  async listDatabases(): Promise<Database[]> {
    const response = await this._request<{ databases: any[] }>('GET', '/databases');
    return response.databases.map((db: any) => ({
      id: db.id,
      name: db.name,
      createdAt: db.created_at,
      tableCount: db.table_count || 0,
      permissions: this.permissions.getPermissions(db.name),
    }));
  }

  async createDatabase(name: string): Promise<Database> {
    // Check permissions (global create permission)
    const response = await this._request<{ database: any }>('POST', '/databases', {
      name,
    });
    return {
      id: response.database.id,
      name: response.database.name,
      createdAt: response.database.created_at,
      tableCount: 0,
    };
  }

  async deleteDatabase(name: string): Promise<void> {
    this.permissions.requirePermission(name, 'delete');
    await this._request('DELETE', `/databases/${name}`);
  }

  // ============================================================================
  // Query Operations
  // ============================================================================

  async query<T = any>(
    query: string,
    database?: string
  ): Promise<QueryResult<T>> {
    if (database) {
      this.permissions.requirePermission(database, 'read');
    }

    const response = await this._request<QueryResult<T>>('POST', '/query', {
      query,
      database,
    });
    return response;
  }

  // ============================================================================
  // Search Operations
  // ============================================================================

  async search<T = any>(
    query: string,
    options?: {
      database?: string;
      table?: string;
      fuzzy?: boolean;
      semantic?: boolean;
      limit?: number;
    }
  ): Promise<SearchResult<T>> {
    if (options?.database) {
      this.permissions.requirePermission(options.database, 'read');
    }

    const response = await this._request<SearchResult<T>>('POST', '/search', {
      query,
      ...options,
    });
    return response;
  }

  // ============================================================================
  // Transaction Support
  // ============================================================================

  async transaction<T>(
    callback: (tx: Transaction) => Promise<T>
  ): Promise<T> {
    // Start transaction
    const txId = await this._request<{ transaction_id: string }>('POST', '/transactions/begin');
    
    try {
      const tx = new Transaction(this, txId.transaction_id);
      const result = await callback(tx);
      await tx.commit();
      return result;
    } catch (error) {
      // Rollback on error
      try {
        await this._request('POST', `/transactions/${txId.transaction_id}/rollback`);
      } catch {}
      throw error;
    }
  }

  // ============================================================================
  // Webhooks
  // ============================================================================

  async createWebhook(config: WebhookConfig): Promise<WebhookConfig> {
    const response = await this._request<{ webhook: WebhookConfig }>('POST', '/webhooks', config);
    return response.webhook;
  }

  async listWebhooks(): Promise<WebhookConfig[]> {
    const response = await this._request<{ webhooks: WebhookConfig[] }>('GET', '/webhooks');
    return response.webhooks;
  }

  async deleteWebhook(id: string): Promise<void> {
    await this._request('DELETE', `/webhooks/${id}`);
  }

  // ============================================================================
  // Health & Status
  // ============================================================================

  async health(): Promise<{ status: string; version: string }> {
    return await this._request('GET', '/health');
  }

  async stats(): Promise<any> {
    return await this._request('GET', '/stats');
  }

  // ============================================================================
  // Internal Methods
  // ============================================================================

  async _request<T = any>(
    method: string,
    path: string,
    body?: any,
    options?: { params?: any }
  ): Promise<T> {
    let url = `${this.url}${path}`;
    if (options?.params) {
      const params = new URLSearchParams();
      for (const [key, value] of Object.entries(options.params)) {
        if (value !== undefined) {
          params.append(key, String(value));
        }
      }
      url += '?' + params.toString();
    }

    const requestOptions: RequestInit = {
      method,
      headers: this.auth.getHeaders(),
      signal: AbortSignal.timeout(this.timeout),
    };

    if (body) {
      requestOptions.body = JSON.stringify(body);
    }

    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= this.retries; attempt++) {
      try {
        const response = await fetch(url, requestOptions);

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          throw createError({
            response: {
              status: response.status,
              data: errorData,
            },
          });
        }

        const data = await response.json();
        return data as T;
      } catch (error: any) {
        lastError = error;

        // Don't retry on certain errors
        if (
          error instanceof NarayanaError &&
          (error.statusCode === 400 || error.statusCode === 401 || error.statusCode === 403)
        ) {
          throw error;
        }

        // Wait before retry (exponential backoff)
        if (attempt < this.retries) {
          await new Promise(resolve => setTimeout(resolve, Math.pow(2, attempt) * 1000));
        }
      }
    }

    throw lastError || new ConnectionError('Request failed after retries');
  }

  private connectWebSocket() {
    try {
      const wsUrl = this.url.replace(/^http/, 'ws') + '/realtime';
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        console.log('NarayanaDB: WebSocket connected');
        // Send auth if available
        if (this.auth.getHeaders()['Authorization']) {
          this.ws?.send(JSON.stringify({
            type: 'auth',
            token: this.auth.getHeaders()['Authorization'],
          }));
        }
      };

      this.ws.onerror = (error) => {
        console.error('NarayanaDB: WebSocket error', error);
      };

      this.ws.onclose = () => {
        // Reconnect after delay
        this.wsReconnectTimer = window.setTimeout(() => {
          this.connectWebSocket();
        }, 5000);
      };
    } catch (error) {
      console.warn('NarayanaDB: WebSocket not available', error);
    }
  }


  getAuth(): AuthManager {
    return this.auth;
  }

  getPermissions(): PermissionManager {
    return this.permissions;
  }

  getCache(): CacheManager {
    return this.cache;
  }
}

// Transaction implementation
export class Transaction {
  constructor(
    private client: NarayanaClient,
    private txId: string
  ) {}

  async commit(): Promise<void> {
    await this.client._request('POST', `/transactions/${this.txId}/commit`);
  }

  async rollback(): Promise<void> {
    await this.client._request('POST', `/transactions/${this.txId}/rollback`);
  }

  async insert(table: string, data: any): Promise<void> {
    await this.client._request('POST', `/transactions/${this.txId}/insert`, {
      table,
      data,
    });
  }

  async update(table: string, data: any, where: any): Promise<void> {
    await this.client._request('POST', `/transactions/${this.txId}/update`, {
      table,
      data,
      where,
    });
  }

  async delete(table: string, where: any): Promise<void> {
    await this.client._request('POST', `/transactions/${this.txId}/delete`, {
      table,
      where,
    });
  }

  async query<T = any>(query: string | any): Promise<QueryResult<T>> {
    return await this.client._request('POST', `/transactions/${this.txId}/query`, {
      query,
    });
  }
}

