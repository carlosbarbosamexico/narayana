// Human Search Integration

import { NarayanaClient } from './client';
import { SearchOptions, SearchResult } from './types';

export class SearchManager {
  constructor(private client: NarayanaClient) {}

  async search<T = any>(
    query: string,
    options?: SearchOptions
  ): Promise<SearchResult<T>> {
    return await this.client._request<SearchResult<T>>('POST', '/search', {
      query,
      ...options,
    });
  }
}

