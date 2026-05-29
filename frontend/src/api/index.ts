import { HttpUsageApi } from './http-usage-api';
import type { UsageApi } from './types';

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

export async function createApi(): Promise<UsageApi> {
  if (isTauri()) {
    const { TauriUsageApi } = await import('./tauri-usage-api');
    return new TauriUsageApi();
  }
  return new HttpUsageApi();
}

export type { UsageApi, HealthResponse, RefreshResponse, Snapshot } from './types';
