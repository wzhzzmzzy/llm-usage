export type Source = 'all' | 'claude' | 'codex' | 'gemini' | 'opencode';
export type ReportType = 'daily' | 'monthly' | 'session' | 'blocks';
export type CellStatus = 'success' | 'stale' | 'error';
export type SnapshotStatus = 'success' | 'partial' | 'error' | 'nodata';
export type HealthStatus = 'healthy' | 'degraded' | 'unhealthy';

export interface ModelBreakdown {
  model: string;
  inputTokens: number;
  cacheReadTokens: number;
  outputTokens: number;
  reasoningTokens: number;
  totalTokens: number;
  requestCount: number;
}

export interface UsageMetric {
  totalTokens: number;
  inputTokens: number;
  cacheReadTokens: number;
  outputTokens: number;
  reasoningTokens: number;
  requestCount?: number;
  modelBreakdown?: ModelBreakdown[];
}

export interface DailyRow {
  date: string;
  totalTokens: number;
  inputTokens: number;
  cacheReadTokens: number;
  outputTokens: number;
  reasoningTokens: number;
  requestCount?: number;
  modelsUsed?: string[];
  modelBreakdown?: ModelBreakdown[];
}

export interface MonthlyRow {
  month: string;
  totalTokens: number;
  inputTokens: number;
  cacheReadTokens: number;
  outputTokens: number;
  reasoningTokens: number;
  requestCount?: number;
  modelsUsed?: string[];
  modelBreakdown?: ModelBreakdown[];
}

export interface SessionRow {
  sessionId: string;
  projectPath?: string;
  totalTokens: number;
  inputTokens: number;
  cacheReadTokens: number;
  outputTokens: number;
  reasoningTokens: number;
  requestCount: number;
  lastActivity?: string;
  modelsUsed?: string[];
  modelBreakdown?: ModelBreakdown[];
}

export interface BlockRow {
  blockId: string;
  startTime: string;
  endTime?: string;
  totalTokens: number;
  inputTokens: number;
  cacheReadTokens: number;
  outputTokens: number;
  reasoningTokens: number;
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

export interface RefreshStatus {
  isRefreshing: boolean;
  lastRefresh?: string;
  lastSuccess?: string;
  lastError?: string;
  snapshotStatus: SnapshotStatus;
}

export interface ModelPricing {
  input: number;
  output: number;
  cacheCreate: number;
  cacheRead: number;
}

export type PricingMap = Record<string, ModelPricing>;

export interface UsageApi {
  health(): Promise<HealthResponse>;
  refresh(): Promise<RefreshStatus>;
  refreshStatus(): Promise<RefreshStatus>;
  getSnapshot(): Promise<Snapshot>;
  getPricing(): Promise<PricingMap>;
}
