// Database Operations - Per-Database Permissions

import { NarayanaClient } from './client';
import { Table, QueryOptions, QueryResult, InsertOptions, UpdateOptions, DeleteOptions } from './types';
import { PermissionError } from './errors';

export class Database {
  constructor(
    private client: NarayanaClient,
    private name: string
  ) {}

  // ============================================================================
  // Table Operations
  // ============================================================================

  async listTables(): Promise<Table[]> {
    this.client.getPermissions().requirePermission(this.name, 'read');
    
    const response = await this.client._request<{ tables: any[] }>('GET', `/databases/${this.name}/tables`);
    return response.tables.map((t: any) => ({
      id: t.id,
      name: t.name,
      schema: t.schema,
      createdAt: t.created_at,
      rowCount: t.row_count || 0,
    }));
  }

  async createTable(name: string, schema: any): Promise<Table> {
    this.client.getPermissions().requirePermission(this.name, 'create');
    
    const response = await this.client._request<{ table: any }>('POST', `/databases/${this.name}/tables`, {
      name,
      schema,
    });
    return {
      id: response.table.id,
      name: response.table.name,
      schema: response.table.schema,
      createdAt: response.table.created_at,
      rowCount: 0,
    };
  }

  async dropTable(name: string): Promise<void> {
    this.client.getPermissions().requirePermission(this.name, 'delete');
    await this.client._request('DELETE', `/databases/${this.name}/tables/${name}`);
  }

  table(name: string): Table {
    this.client.getPermissions().requirePermission(this.name, 'read');
    return new Table(this.client, this.name, name);
  }

  // ============================================================================
  // Query Operations
  // ============================================================================

  async query<T = any>(query: string | QueryOptions): Promise<QueryResult<T>> {
    this.client.getPermissions().requirePermission(this.name, 'read');
    
    if (typeof query === 'string') {
      return await this.client.query<T>(query, this.name);
    } else {
      return await this.client._request<QueryResult<T>>('POST', `/databases/${this.name}/query`, query);
    }
  }

  // ============================================================================
  // Search Operations
  // ============================================================================

  async search<T = any>(query: string, options?: {
    table?: string;
    fuzzy?: boolean;
    semantic?: boolean;
    limit?: number;
  }): Promise<any> {
    this.client.getPermissions().requirePermission(this.name, 'read');
    return await this.client.search<T>(query, { ...options, database: this.name });
  }

  // ============================================================================
  // Batch Operations
  // ============================================================================

  async batch(operations: any[]): Promise<any> {
    this.client.getPermissions().requirePermission(this.name, 'write');
    return await this.client._request('POST', `/databases/${this.name}/batch`, { operations });
  }

  // ============================================================================
  // Transaction
  // ============================================================================

  async transaction<T>(callback: (tx: any) => Promise<T>): Promise<T> {
    this.client.getPermissions().requirePermission(this.name, 'write');
    return await this.client.transaction(async (tx) => {
      // Set database context for transaction
      return await callback(tx);
    });
  }
}

