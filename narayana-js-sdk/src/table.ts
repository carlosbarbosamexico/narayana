// Table Operations - Elegant API

import { NarayanaClient } from './client';
import {
  QueryOptions,
  QueryResult,
  InsertOptions,
  UpdateOptions,
  DeleteOptions,
  SearchOptions,
  SearchResult,
  RealtimeSubscription,
  StreamOptions,
} from './types';

export class Table {
  constructor(
    private client: NarayanaClient,
    private database: string,
    private name: string
  ) {}

  // ============================================================================
  // Query Operations
  // ============================================================================

  async select<T = any>(options?: QueryOptions): Promise<QueryResult<T>> {
    this.client.getPermissions().requirePermission(this.database, 'read');
    
    return await this.client._request<QueryResult<T>>('GET', `/databases/${this.database}/tables/${this.name}`, undefined, {
      params: options,
    });
  }

  async find<T = any>(where: any, options?: { limit?: number; offset?: number }): Promise<QueryResult<T>> {
    return await this.select<T>({
      where: this.normalizeWhere(where),
      ...options,
    });
  }

  async findOne<T = any>(where: any): Promise<T | null> {
    const result = await this.find<T>(where, { limit: 1 });
    return result.rows[0] || null;
  }

  async findById<T = any>(id: string | number): Promise<T | null> {
    return await this.findOne<T>({ id });
  }

  // ============================================================================
  // Insert Operations
  // ============================================================================

  async insert<T = any>(data: T | T[], options?: InsertOptions): Promise<{ inserted: number; ids?: string[] }> {
    this.client.getPermissions().requirePermission(this.database, 'write');
    
    const response = await this.client._request<{ inserted: number; ids?: string[] }>(
      'POST',
      `/databases/${this.database}/tables/${this.name}/insert`,
      {
        data: Array.isArray(data) ? data : [data],
        ...options,
      }
    );
    return response;
  }

  async insertOne<T = any>(data: T): Promise<{ id: string }> {
    const result = await this.insert(data, { returnIds: true });
    return { id: result.ids![0] };
  }

  // ============================================================================
  // Update Operations
  // ============================================================================

  async update<T = any>(data: Partial<T>, where: any, options?: UpdateOptions): Promise<{ updated: number }> {
    this.client.getPermissions().requirePermission(this.database, 'write');
    
    const response = await this.client._request<{ updated: number }>(
      'POST',
      `/databases/${this.database}/tables/${this.name}/update`,
      {
        data,
        where: this.normalizeWhere(where),
        ...options,
      }
    );
    return response;
  }

  async updateById<T = any>(id: string | number, data: Partial<T>): Promise<{ updated: number }> {
    return await this.update(data, { id });
  }

  async upsert<T = any>(data: T, where: any): Promise<{ inserted: number; updated: number }> {
    this.client.getPermissions().requirePermission(this.database, 'write');
    
    const response = await this.client._request<{ inserted: number; updated: number }>(
      'POST',
      `/databases/${this.database}/tables/${this.name}/upsert`,
      {
        data,
        where: this.normalizeWhere(where),
      }
    );
    return response;
  }

  // ============================================================================
  // Delete Operations
  // ============================================================================

  async delete(where: any, options?: DeleteOptions): Promise<{ deleted: number }> {
    this.client.getPermissions().requirePermission(this.database, 'delete');
    
    const response = await this.client._request<{ deleted: number }>(
      'DELETE',
      `/databases/${this.database}/tables/${this.name}`,
      {
        where: this.normalizeWhere(where),
        ...options,
      }
    );
    return response;
  }

  async deleteById(id: string | number): Promise<{ deleted: number }> {
    return await this.delete({ id });
  }

  async deleteMany(ids: (string | number)[]): Promise<{ deleted: number }> {
    return await this.delete({ id: { operator: 'IN', value: ids } });
  }

  // ============================================================================
  // Search Operations
  // ============================================================================

  async search<T = any>(query: string, options?: SearchOptions): Promise<SearchResult<T>> {
    this.client.getPermissions().requirePermission(this.database, 'read');
    
    return await this.client._request<SearchResult<T>>(
      'POST',
      `/databases/${this.database}/tables/${this.name}/search`,
      {
        query,
        ...options,
      }
    );
  }

  // ============================================================================
  // Count Operations
  // ============================================================================

  async count(where?: any): Promise<number> {
    this.client.getPermissions().requirePermission(this.database, 'read');
    
    const response = await this.client._request<{ count: number }>(
      'POST',
      `/databases/${this.database}/tables/${this.name}/count`,
      where ? { where: this.normalizeWhere(where) } : undefined
    );
    return response.count;
  }

  // ============================================================================
  // Aggregation Operations
  // ============================================================================

  async aggregate(options: {
    groupBy?: string[];
    aggregations: Array<{ function: string; column: string; alias?: string }>;
    where?: any;
  }): Promise<QueryResult> {
    this.client.getPermissions().requirePermission(this.database, 'read');
    
    return await this.client._request<QueryResult>(
      'POST',
      `/databases/${this.database}/tables/${this.name}/aggregate`,
      {
        ...options,
        where: options.where ? this.normalizeWhere(options.where) : undefined,
      }
    );
  }

  // ============================================================================
  // Realtime Operations
  // ============================================================================

  subscribe(callback: (data: any) => void): RealtimeSubscription {
    this.client.getPermissions().requirePermission(this.database, 'read');
    
    // In production, would use WebSocket
    return {
      unsubscribe: () => {},
      on: (event: string, cb: (data: any) => void) => {
        // WebSocket event handling
      },
    };
  }

  // ============================================================================
  // Streaming Operations
  // ============================================================================

  async stream<T = any>(
    options?: QueryOptions & StreamOptions,
    onData?: (row: T) => void
  ): Promise<void> {
    this.client.getPermissions().requirePermission(this.database, 'read');
    
    // In production, would use streaming API
    const result = await this.select<T>(options);
    for (const row of result.rows) {
      if (onData) {
        onData(row);
      }
    }
  }

  // ============================================================================
  // Helper Methods
  // ============================================================================

  private normalizeWhere(where: any): any {
    // Normalize where clause to API format
    if (typeof where === 'object' && where !== null) {
      // Simple object format: { id: 123 } -> { column: 'id', operator: '=', value: 123 }
      if (!where.column && !where.operator) {
        const normalized: any = {};
        for (const [key, value] of Object.entries(where)) {
          if (Array.isArray(normalized.and)) {
            normalized.and.push({ column: key, operator: '=', value });
          } else {
            normalized.and = [{ column: key, operator: '=', value }];
          }
        }
        return normalized;
      }
    }
    return where;
  }
}

