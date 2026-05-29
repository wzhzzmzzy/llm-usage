import { invoke } from '@tauri-apps/api/tauri';
import type { HealthResponse, RefreshStatus, Snapshot, UsageApi } from './types';

export class TauriUsageApi implements UsageApi {
  async health(): Promise<HealthResponse> {
    return invoke('health');
  }

  async refresh(): Promise<RefreshStatus> {
    return invoke('refresh');
  }

  async refreshStatus(): Promise<RefreshStatus> {
    return invoke('refresh_status');
  }

  async getSnapshot(): Promise<Snapshot> {
    return invoke('get_snapshot');
  }
}
