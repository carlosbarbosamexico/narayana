// Session Management - Built-in Session Support
// No backend needed - NarayanaDB handles everything

export interface Session {
  id: string;
  userId: string;
  expiresAt: number;
  data: Record<string, any>;
  createdAt: number;
  lastAccessed: number;
}

export interface SessionConfig {
  ttl?: number; // Time to live in seconds
  autoRefresh?: boolean;
  storage?: 'memory' | 'localStorage' | 'sessionStorage';
}

export class SessionManager {
  private session: Session | null = null;
  private config: SessionConfig;
  private refreshTimer?: number;

  constructor(config: SessionConfig = {}) {
    this.config = {
      ttl: config.ttl || 3600, // 1 hour default
      autoRefresh: config.autoRefresh ?? true,
      storage: config.storage || 'localStorage',
    };

    // Load session from storage
    this.loadSession();
  }

  // ============================================================================
  // Session Operations
  // ============================================================================

  async create(userId: string, data: Record<string, any> = {}): Promise<Session> {
    const session: Session = {
      id: this.generateSessionId(),
      userId,
      expiresAt: Date.now() + (this.config.ttl! * 1000),
      data,
      createdAt: Date.now(),
      lastAccessed: Date.now(),
    };

    this.session = session;
    this.saveSession();
    
    if (this.config.autoRefresh) {
      this.startAutoRefresh();
    }

    return session;
  }

  async get(): Promise<Session | null> {
    if (!this.session) {
      this.loadSession();
    }

    if (this.session && this.isExpired(this.session)) {
      await this.destroy();
      return null;
    }

    if (this.session) {
      this.session.lastAccessed = Date.now();
      this.saveSession();
    }

    return this.session;
  }

  async update(data: Partial<Record<string, any>>): Promise<void> {
    if (!this.session) {
      throw new Error('No active session');
    }

    this.session.data = { ...this.session.data, ...data };
    this.session.lastAccessed = Date.now();
    this.saveSession();
  }

  async getData(key: string): Promise<any> {
    const session = await this.get();
    return session?.data[key];
  }

  async setData(key: string, value: any): Promise<void> {
    if (!this.session) {
      throw new Error('No active session');
    }

    this.session.data[key] = value;
    this.session.lastAccessed = Date.now();
    this.saveSession();
  }

  async destroy(): Promise<void> {
    this.session = null;
    this.stopAutoRefresh();
    this.clearStorage();
  }

  async refresh(): Promise<Session> {
    if (!this.session) {
      throw new Error('No active session');
    }

    this.session.expiresAt = Date.now() + (this.config.ttl! * 1000);
    this.session.lastAccessed = Date.now();
    this.saveSession();

    return this.session;
  }

  isActive(): boolean {
    if (!this.session) {
      return false;
    }

    return !this.isExpired(this.session);
  }

  // ============================================================================
  // Storage Management
  // ============================================================================

  private saveSession(): void {
    if (!this.session) {
      return;
    }

    const key = 'narayana_session';
    const data = JSON.stringify(this.session);

    switch (this.config.storage) {
      case 'localStorage':
        try {
          localStorage.setItem(key, data);
        } catch (e) {
          console.warn('Failed to save session to localStorage', e);
        }
        break;
      case 'sessionStorage':
        try {
          sessionStorage.setItem(key, data);
        } catch (e) {
          console.warn('Failed to save session to sessionStorage', e);
        }
        break;
      case 'memory':
        // Already in memory
        break;
    }
  }

  private loadSession(): void {
    const key = 'narayana_session';
    let data: string | null = null;

    switch (this.config.storage) {
      case 'localStorage':
        try {
          data = localStorage.getItem(key);
        } catch (e) {
          console.warn('Failed to load session from localStorage', e);
        }
        break;
      case 'sessionStorage':
        try {
          data = sessionStorage.getItem(key);
        } catch (e) {
          console.warn('Failed to load session from sessionStorage', e);
        }
        break;
      case 'memory':
        // Already in memory
        break;
    }

    if (data) {
      try {
        this.session = JSON.parse(data);
        if (this.isExpired(this.session!)) {
          this.session = null;
          this.clearStorage();
        } else if (this.config.autoRefresh) {
          this.startAutoRefresh();
        }
      } catch (e) {
        console.warn('Failed to parse session data', e);
        this.session = null;
        this.clearStorage();
      }
    }
  }

  private clearStorage(): void {
    const key = 'narayana_session';

    switch (this.config.storage) {
      case 'localStorage':
        try {
          localStorage.removeItem(key);
        } catch (e) {
          console.warn('Failed to clear session from localStorage', e);
        }
        break;
      case 'sessionStorage':
        try {
          sessionStorage.removeItem(key);
        } catch (e) {
          console.warn('Failed to clear session from sessionStorage', e);
        }
        break;
      case 'memory':
        // Already cleared
        break;
    }
  }

  // ============================================================================
  // Auto-Refresh
  // ============================================================================

  private startAutoRefresh(): void {
    this.stopAutoRefresh();

    // Refresh session 5 minutes before expiry
    const refreshInterval = Math.max((this.config.ttl! - 300) * 1000, 60000);
    
    this.refreshTimer = window.setInterval(() => {
      if (this.session && !this.isExpired(this.session)) {
        this.refresh().catch(console.error);
      } else {
        this.stopAutoRefresh();
      }
    }, refreshInterval);
  }

  private stopAutoRefresh(): void {
    if (this.refreshTimer) {
      clearInterval(this.refreshTimer);
      this.refreshTimer = undefined;
    }
  }

  // ============================================================================
  // Helpers
  // ============================================================================

  private isExpired(session: Session): boolean {
    return Date.now() >= session.expiresAt;
  }

  private generateSessionId(): string {
    return `sess_${Date.now()}_${Math.random().toString(36).substring(2, 15)}`;
  }
}

