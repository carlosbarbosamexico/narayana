// Authentication and Authorization for NarayanaDB SDK
// Built-in Authentication - No backend needed!

import { NarayanaError, AuthenticationError, PermissionError } from './errors';
import { DatabasePermissions } from './types';
import { SessionManager } from './session';

export interface AuthConfig {
  apiKey?: string;
  token?: string;
  username?: string;
  password?: string;
  sessionStorage?: 'memory' | 'localStorage' | 'sessionStorage';
}

export interface User {
  id: string;
  email: string;
  name?: string;
  role?: string;
  permissions?: Record<string, DatabasePermissions>;
}

export interface AuthResult {
  user: User;
  token: string;
  session: any;
}

export class AuthManager {
  private apiKey?: string;
  private token?: string;
  private permissions: Map<string, DatabasePermissions> = new Map();
  private sessionManager: SessionManager;
  private currentUser: User | null = null;

  constructor(config?: AuthConfig) {
    if (config?.apiKey) {
      this.apiKey = config.apiKey;
    }
    if (config?.token) {
      this.token = config.token;
    }

    // Initialize session manager
    this.sessionManager = new SessionManager({
      storage: config?.sessionStorage || 'localStorage',
      autoRefresh: true,
    });
  }

  setApiKey(apiKey: string) {
    this.apiKey = apiKey;
  }

  setToken(token: string) {
    this.token = token;
  }

  setPermissions(database: string, permissions: DatabasePermissions) {
    this.permissions.set(database, permissions);
  }

  getHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    return headers;
  }

  checkPermission(
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
    if (!this.checkPermission(database, permission)) {
      throw new PermissionError(
        `Permission '${permission}' required for database '${database}'`,
        database,
        permission
      );
    }
  }

  async authenticate(url: string, credentials: {
    username?: string;
    email?: string;
    password: string;
  }): Promise<AuthResult> {
    const response = await fetch(`${url}/auth/login`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        email: credentials.email || credentials.username,
        password: credentials.password,
      }),
    });

    if (!response.ok) {
      throw new AuthenticationError('Authentication failed');
    }

    const data = await response.json();
    this.token = data.token;
    this.currentUser = data.user;

    // Create session
    const session = await this.sessionManager.create(data.user.id, {
      user: data.user,
      token: data.token,
    });

    // Set permissions from user
    if (data.user.permissions) {
      for (const [database, perms] of Object.entries(data.user.permissions)) {
        this.setPermissions(database, perms as DatabasePermissions);
      }
    }

    return {
      user: data.user,
      token: data.token,
      session,
    };
  }

  async register(url: string, userData: {
    email: string;
    password: string;
    name?: string;
  }): Promise<AuthResult> {
    const response = await fetch(`${url}/auth/register`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(userData),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({}));
      throw new AuthenticationError(error.message || 'Registration failed');
    }

    const data = await response.json();
    this.token = data.token;
    this.currentUser = data.user;

    // Create session
    const session = await this.sessionManager.create(data.user.id, {
      user: data.user,
      token: data.token,
    });

    // Set permissions from user
    if (data.user.permissions) {
      for (const [database, perms] of Object.entries(data.user.permissions)) {
        this.setPermissions(database, perms as DatabasePermissions);
      }
    }

    return {
      user: data.user,
      token: data.token,
      session,
    };
  }

  async logout(url?: string): Promise<void> {
    // Call logout endpoint if URL provided
    if (url && this.token) {
      try {
        await fetch(`${url}/auth/logout`, {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${this.token}`,
          },
        });
      } catch (e) {
        // Ignore errors
      }
    }

    // Clear local state
    this.token = undefined;
    this.currentUser = null;
    this.permissions.clear();
    await this.sessionManager.destroy();
  }

  async getCurrentUser(): Promise<User | null> {
    // Try to get from session first
    const session = await this.sessionManager.get();
    if (session && session.data.user) {
      this.currentUser = session.data.user;
      this.token = session.data.token;
      return this.currentUser;
    }

    return this.currentUser;
  }

  async isAuthenticated(): Promise<boolean> {
    const session = await this.sessionManager.get();
    return session !== null && session !== undefined;
  }

  getSessionManager(): SessionManager {
    return this.sessionManager;
  }

  async refreshToken(url: string): Promise<string> {
    if (!this.token) {
      throw new AuthenticationError('No token to refresh');
    }

    const response = await fetch(`${url}/auth/refresh`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.token}`,
      },
    });

    if (!response.ok) {
      throw new AuthenticationError('Token refresh failed');
    }

    const data = await response.json();
    this.token = data.token;
    return data.token;
  }
}

