// Client-Side Caching - Built-in Caching Support
// No backend needed - NarayanaDB handles everything

export interface CacheConfig {
  ttl?: number; // Time to live in seconds
  maxSize?: number; // Maximum cache size
  storage?: 'memory' | 'localStorage' | 'indexedDB';
  strategy?: 'lru' | 'fifo' | 'lfu';
}

export interface CacheEntry<T> {
  key: string;
  value: T;
  expiresAt: number;
  accessedAt: number;
  accessCount: number;
}

export class CacheManager {
  private cache: Map<string, CacheEntry<any>> = new Map();
  private config: CacheConfig;

  constructor(config: CacheConfig = {}) {
    this.config = {
      ttl: config.ttl || 300, // 5 minutes default
      maxSize: config.maxSize || 1000,
      storage: config.storage || 'memory',
      strategy: config.strategy || 'lru',
    };

    // Load cache from storage
    this.loadCache();

    // Start cleanup interval
    this.startCleanup();
  }

  // ============================================================================
  // Cache Operations
  // ============================================================================

  async get<T = any>(key: string): Promise<T | null> {
    const entry = this.cache.get(key);

    if (!entry) {
      return null;
    }

    // Check if expired
    if (Date.now() >= entry.expiresAt) {
      await this.delete(key);
      return null;
    }

    // Update access info
    entry.accessedAt = Date.now();
    entry.accessCount += 1;
    this.cache.set(key, entry);
    this.saveCache();

    return entry.value as T;
  }

  async set<T = any>(key: string, value: T, ttl?: number): Promise<void> {
    const expiresAt = Date.now() + ((ttl || this.config.ttl!) * 1000);

    // Check if cache is full
    if (this.cache.size >= this.config.maxSize!) {
      await this.evict();
    }

    const entry: CacheEntry<T> = {
      key,
      value,
      expiresAt,
      accessedAt: Date.now(),
      accessCount: 0,
    };

    this.cache.set(key, entry);
    this.saveCache();
  }

  async delete(key: string): Promise<void> {
    this.cache.delete(key);
    this.saveCache();
  }

  async clear(): Promise<void> {
    this.cache.clear();
    this.clearStorage();
  }

  async has(key: string): Promise<boolean> {
    const entry = this.cache.get(key);
    if (!entry) {
      return false;
    }

    if (Date.now() >= entry.expiresAt) {
      await this.delete(key);
      return false;
    }

    return true;
  }

  async keys(): Promise<string[]> {
    // Clean expired entries
    await this.cleanup();
    return Array.from(this.cache.keys());
  }

  async size(): Promise<number> {
    await this.cleanup();
    return this.cache.size;
  }

  // ============================================================================
  // Cache with Function
  // ============================================================================

  async getOrSet<T = any>(
    key: string,
    fetcher: () => Promise<T>,
    ttl?: number
  ): Promise<T> {
    const cached = await this.get<T>(key);
    if (cached !== null) {
      return cached;
    }

    const value = await fetcher();
    await this.set(key, value, ttl);
    return value;
  }

  // ============================================================================
  // Eviction
  // ============================================================================

  private async evict(): Promise<void> {
    if (this.cache.size < this.config.maxSize!) {
      return;
    }

    let entryToEvict: string | null = null;

    switch (this.config.strategy) {
      case 'lru':
        // Least recently used
        entryToEvict = Array.from(this.cache.entries())
          .sort((a, b) => a[1].accessedAt - b[1].accessedAt)[0][0];
        break;

      case 'lfu':
        // Least frequently used
        entryToEvict = Array.from(this.cache.entries())
          .sort((a, b) => a[1].accessCount - b[1].accessCount)[0][0];
        break;

      case 'fifo':
        // First in, first out
        entryToEvict = Array.from(this.cache.keys())[0];
        break;
    }

    if (entryToEvict) {
      await this.delete(entryToEvict);
    }
  }

  // ============================================================================
  // Cleanup
  // ============================================================================

  private async cleanup(): Promise<void> {
    const now = Date.now();
    const keysToDelete: string[] = [];

    for (const [key, entry] of this.cache.entries()) {
      if (now >= entry.expiresAt) {
        keysToDelete.push(key);
      }
    }

    for (const key of keysToDelete) {
      await this.delete(key);
    }

    if (keysToDelete.length > 0) {
      this.saveCache();
    }
  }

  private startCleanup(): void {
    // Cleanup every minute
    setInterval(() => {
      this.cleanup().catch(console.error);
    }, 60000);
  }

  // ============================================================================
  // Storage Management
  // ============================================================================

  private saveCache(): void {
    if (this.config.storage === 'memory') {
      return;
    }

    const key = 'narayana_cache';
    const data: Array<[string, CacheEntry<any>]> = Array.from(this.cache.entries());

    try {
      if (this.config.storage === 'localStorage') {
        // Limit size for localStorage (5MB limit)
        const json = JSON.stringify(data.slice(0, 100));
        localStorage.setItem(key, json);
      } else if (this.config.storage === 'indexedDB') {
        // Use IndexedDB for larger caches
        this.saveToIndexedDB(data).catch(console.error);
      }
    } catch (e) {
      console.warn('Failed to save cache', e);
    }
  }

  private loadCache(): void {
    if (this.config.storage === 'memory') {
      return;
    }

    const key = 'narayana_cache';
    let data: Array<[string, CacheEntry<any>]> | null = null;

    try {
      if (this.config.storage === 'localStorage') {
        const json = localStorage.getItem(key);
        if (json) {
          data = JSON.parse(json);
        }
      } else if (this.config.storage === 'indexedDB') {
        // Load from IndexedDB
        this.loadFromIndexedDB().then(loaded => {
          if (loaded) {
            this.cache = new Map(loaded);
          }
        }).catch(console.error);
        return;
      }
    } catch (e) {
      console.warn('Failed to load cache', e);
    }

    if (data) {
      const now = Date.now();
      for (const [key, entry] of data) {
        // Only load non-expired entries
        if (entry.expiresAt > now) {
          this.cache.set(key, entry);
        }
      }
    }
  }

  private clearStorage(): void {
    const key = 'narayana_cache';

    try {
      if (this.config.storage === 'localStorage') {
        localStorage.removeItem(key);
      } else if (this.config.storage === 'indexedDB') {
        this.clearIndexedDB().catch(console.error);
      }
    } catch (e) {
      console.warn('Failed to clear cache storage', e);
    }
  }

  private async saveToIndexedDB(data: Array<[string, CacheEntry<any>]>): Promise<void> {
    // IndexedDB implementation would go here
    // For now, fallback to localStorage
    try {
      const json = JSON.stringify(data.slice(0, 100));
      localStorage.setItem('narayana_cache', json);
    } catch (e) {
      console.warn('IndexedDB not available, using localStorage', e);
    }
  }

  private async loadFromIndexedDB(): Promise<Array<[string, CacheEntry<any>]> | null> {
    // IndexedDB implementation would go here
    try {
      const json = localStorage.getItem('narayana_cache');
      return json ? JSON.parse(json) : null;
    } catch (e) {
      return null;
    }
  }

  private async clearIndexedDB(): Promise<void> {
    // IndexedDB implementation would go here
    localStorage.removeItem('narayana_cache');
  }
}

