// Per-Database Permissions System

import { DatabasePermissions } from './types';
import { PermissionError } from './errors';

export class PermissionManager {
  private permissions: Map<string, DatabasePermissions> = new Map();

  setPermissions(database: string, permissions: DatabasePermissions) {
    this.permissions.set(database, permissions);
  }

  getPermissions(database: string): DatabasePermissions | undefined {
    return this.permissions.get(database);
  }

  hasPermission(
    database: string,
    permission: keyof DatabasePermissions
  ): boolean {
    const dbPermissions = this.permissions.get(database);
    if (!dbPermissions) {
      return false;
    }

    // Admin has all permissions
    if (dbPermissions.admin) {
      return true;
    }

    return dbPermissions[permission] === true;
  }

  requirePermission(
    database: string,
    permission: keyof DatabasePermissions
  ): void {
    if (!this.hasPermission(database, permission)) {
      throw new PermissionError(
        `Permission '${permission}' required for database '${database}'`,
        database,
        permission
      );
    }
  }

  canRead(database: string): boolean {
    return this.hasPermission(database, 'read');
  }

  canWrite(database: string): boolean {
    return this.hasPermission(database, 'write');
  }

  canCreate(database: string): boolean {
    return this.hasPermission(database, 'create');
  }

  canDelete(database: string): boolean {
    return this.hasPermission(database, 'delete');
  }

  isAdmin(database: string): boolean {
    return this.hasPermission(database, 'admin');
  }
}

