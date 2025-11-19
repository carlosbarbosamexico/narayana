// Webhook Management

import { NarayanaClient } from './client';
import { WebhookConfig } from './types';

export class WebhookManager {
  constructor(private client: NarayanaClient) {}

  async create(config: WebhookConfig): Promise<WebhookConfig> {
    return await this.client.createWebhook(config);
  }

  async list(): Promise<WebhookConfig[]> {
    return await this.client.listWebhooks();
  }

  async delete(id: string): Promise<void> {
    return await this.client.deleteWebhook(id);
  }
}

