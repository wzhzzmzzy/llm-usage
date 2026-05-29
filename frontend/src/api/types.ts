export type Source = 'all' | 'claude' | 'codex' | 'opencode';
export type ReportType = 'daily' | 'monthly' | 'session' | 'blocks';
export type CellStatus = 'success' | 'stale' | 'error';
export type SnapshotStatus = 'success' | 'partial' | 'error' | 'nodata';
export type HealthStatus = 'healthy' | 'degraded' | 'unhealthy';

export interface UsageMetric {
  totalCostUsd: string;
  totalCostUsdNumber: number;
  costFormatted: string;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens?: number;
  cacheReadTokens?: number;
  requestCount?: number;
  modelBreakdown?: ModelUsage[];
}

export interface ModelUsage {
  model: string;
  totalCostUsd: string;
  totalCostUsdNumber: number;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
}

export interface DailyRow {
  date: string;
  costUsd: string;
  costUsdNumber: number;
  costFormatted: string;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens?: number;
  cacheReadTokens?: number;
  requestCount?: number;
  modelsUsed?: string[];
}

export interface MonthlyRow {
  month: string;
  costUsd: string;
  costUsdNumber: number;
  costFormatted: string;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens?: number;
  cacheReadTokens?: number;
  requestCount?: number;
  modelsUsed?: string[];
}

export interface SessionRow {
  sessionId: string;
  projectPath?: string;
  costUsd: string;
  costUsdNumber: number;
  costFormatted: string;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  lastActivity?: string;
  modelsUsed?: string[];
}

export interface BlockRow {
  blockId: string;
  startTime: string;
  endTime?: string;
  costUsd: string;
  costUsdNumber: number;
  costFormatted: string;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  isActive: boolean;
  modelsUsed?: string[];
}

export interface DailyReport {
  days: DailyRow[];
  totals: UsageMetric;
}

export interface MonthlyReport {
  months: MonthlyRow[];
  totals: UsageMetric;
}

export interface SessionReport {
  sessions: SessionRow[];
  totals: UsageMetric;
}

export interface BlocksReport {
  blocks: BlockRow[];
  activeBlock?: BlockRow;
  totals: UsageMetric;
}

export interface CellResult {
  cell: { source: Source; report: ReportType };
  status: CellStatus;
  error?: string;
  stderr?: string;
  exitCode?: number;
  durationMs: number;
}

export interface Snapshot {
  status: SnapshotStatus;
  lastRefresh?: string;
  lastSuccess?: string;
  lastError?: string;
  cells: Record<string, CellResult>;
  daily: Record<string, DailyReport>;
  monthly: Record<string, MonthlyReport>;
  session: Record<string, SessionReport>;
  blocks: Record<string, BlocksReport>;
  timezone: string;
}

export interface HealthResponse {
  status: HealthStatus;
  runner: {
    found: boolean;
    path?: string;
    version?: string;
  };
  ccusage: {
    available: boolean;
    version?: string;
  };
  configPath: string;
}

export interface RefreshResponse {
  status: SnapshotStatus;
  snapshot: Snapshot;
}

export interface UsageApi {
  health(): Promise<HealthResponse>;
  refresh(): Promise<RefreshResponse>;
  getSnapshot(): Promise<Snapshot>;
}
