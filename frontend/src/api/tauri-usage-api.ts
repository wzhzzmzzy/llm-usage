import { invoke } from '@tauri-apps/api/tauri';
import type { HealthResponse, RefreshResponse, Snapshot, UsageApi } from './types';

export class TauriUsageApi implements UsageApi {
  async health(): Promise<HealthResponse> {
    return invoke('health');
  }

  async refresh(): Promise<RefreshResponse> {
    return invoke('refresh');
  }

  async getSnapshot(): Promise<Snapshot> {
    return invoke('get_snapshot');
  }
}
