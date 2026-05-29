import type { HealthResponse, RefreshResponse, Snapshot, UsageApi } from './types';

export class HttpUsageApi implements UsageApi {
  private baseUrl: string;

  constructor(baseUrl: string = '') {
    this.baseUrl = baseUrl;
  }

  async health(): Promise<HealthResponse> {
    const res = await fetch(`${this.baseUrl}/api/health`);
    if (!res.ok) throw new Error(`Health check failed: ${res.status}`);
    return res.json();
  }

  async refresh(): Promise<RefreshResponse> {
    const res = await fetch(`${this.baseUrl}/api/refresh`, { method: 'POST' });
    if (!res.ok) throw new Error(`Refresh failed: ${res.status}`);
    return res.json();
  }

  async getSnapshot(): Promise<Snapshot> {
    const res = await fetch(`${this.baseUrl}/api/snapshot`);
    if (!res.ok) throw new Error(`Get snapshot failed: ${res.status}`);
    return res.json();
  }
}
