// Batch Operations

import { BatchOperation, BatchResult } from './types';
import { NarayanaClient } from './client';

export class BatchManager {
  constructor(private client: NarayanaClient) {}

  async execute(operations: BatchOperation[]): Promise<BatchResult> {
    const response = await this.client._request<BatchResult>('POST', '/batch', {
      operations,
    });
    return response;
  }
}

