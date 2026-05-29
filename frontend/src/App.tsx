import { useState, useEffect, useCallback, useRef } from 'react';
import { HttpUsageApi } from './api/http-usage-api';
import type { Snapshot, Source, ReportType, UsageApi, RefreshStatus } from './api/types';
import { ContributionCalendar } from './components/contribution-calendar';
import { UsageTable } from './components/usage-table';

let tauriInvoke: ((cmd: string, args?: Record<string, unknown>) => Promise<any>) | null = null;

async function getTauriInvoke() {
  if (tauriInvoke) return tauriInvoke;
  try {
    const { invoke } = await import('@tauri-apps/api/tauri');
    tauriInvoke = invoke;
    return invoke;
  } catch {
    return null;
  }
}

function createApi(): UsageApi {
  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
  if (isTauri) {
    return {
      async health() {
        const invoke = await getTauriInvoke();
        if (!invoke) throw new Error('Tauri not available');
        return invoke('health');
      },
      async refresh() {
        const invoke = await getTauriInvoke();
        if (!invoke) throw new Error('Tauri not available');
        return invoke('refresh');
      },
      async refreshStatus() {
        const invoke = await getTauriInvoke();
        if (!invoke) throw new Error('Tauri not available');
        return invoke('refresh_status');
      },
      async getSnapshot() {
        const invoke = await getTauriInvoke();
        if (!invoke) throw new Error('Tauri not available');
        return invoke('get_snapshot');
      },
    };
  }
  return new HttpUsageApi();
}

const api = createApi();

const SOURCES: Source[] = ['all', 'claude', 'codex', 'opencode'];
const TABS: { key: ReportType; label: string }[] = [
  { key: 'daily', label: 'Daily' },
  { key: 'monthly', label: 'Monthly' },
  { key: 'session', label: 'Sessions' },
  { key: 'blocks', label: 'Blocks' },
];

function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [refreshStatus, setRefreshStatus] = useState<RefreshStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [source, setSource] = useState<Source>('all');
  const [tab, setTab] = useState<ReportType>('daily');
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const hasRefreshed = useRef(false);
  const pollIntervalRef = useRef<number | null>(null);

  const loadSnapshot = useCallback(async () => {
    try {
      const snap = await api.getSnapshot();
      setSnapshot(snap);
    } catch (e) {
      console.error('Failed to load snapshot:', e);
    }
  }, []);

  const pollRefreshStatus = useCallback(async () => {
    try {
      const status = await api.refreshStatus();
      setRefreshStatus(status);

      if (!status.isRefreshing) {
        if (pollIntervalRef.current) {
          clearInterval(pollIntervalRef.current);
          pollIntervalRef.current = null;
        }
        setLoading(false);
        await loadSnapshot();

        if (status.lastError) {
          setError(status.lastError);
        }
      }
    } catch (e) {
      console.error('Failed to poll refresh status:', e);
    }
  }, [loadSnapshot]);

  const handleRefresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const status = await api.refresh();
      setRefreshStatus(status);

      if (status.isRefreshing) {
        pollIntervalRef.current = window.setInterval(pollRefreshStatus, 1000);
      } else {
        setLoading(false);
        await loadSnapshot();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Refresh failed');
      setLoading(false);
    }
  }, [loadSnapshot, pollRefreshStatus]);

  useEffect(() => {
    if (!hasRefreshed.current) {
      hasRefreshed.current = true;
      loadSnapshot().then(() => {
        handleRefresh();
      });
    }

    return () => {
      if (pollIntervalRef.current) {
        clearInterval(pollIntervalRef.current);
      }
    };
  }, [loadSnapshot, handleRefresh]);

  const dailyData = snapshot?.daily?.[source === 'all' ? 'all_daily' : `${source}_daily`];
  const monthlyData = snapshot?.monthly?.[source === 'all' ? 'all_monthly' : `${source}_monthly`];
  const sessionData = snapshot?.session?.[source === 'all' ? 'all_session' : `${source}_session`];
  const blocksData = snapshot?.blocks?.[source === 'all' ? 'all_blocks' : `${source}_blocks`];

  const tableData = tab === 'daily' ? dailyData?.days ?? []
    : tab === 'monthly' ? monthlyData?.months ?? []
    : tab === 'session' ? sessionData?.sessions ?? []
    : blocksData?.blocks ?? [];

  const totals = tab === 'daily' ? dailyData?.totals
    : tab === 'monthly' ? monthlyData?.totals
    : tab === 'session' ? sessionData?.totals
    : blocksData?.totals;

  const calendarData = source === 'all'
    ? (snapshot?.daily ?? {})
    : { [`${source}_daily`]: snapshot?.daily?.[`${source}_daily`] };

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b px-6 py-4">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">LLM Usage Dashboard</h1>
          <div className="flex items-center gap-4">
            {snapshot?.lastSuccess && (
              <span className="text-sm text-muted-foreground">
                Last updated: {new Date(snapshot.lastSuccess).toLocaleString()}
              </span>
            )}
            <button
              onClick={handleRefresh}
              disabled={loading}
              className="inline-flex items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {loading ? 'Refreshing...' : 'Refresh'}
            </button>
          </div>
        </div>

        {error && (
          <div className="mt-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
            {error}
          </div>
        )}

        {loading && refreshStatus?.isRefreshing && (
          <div className="mt-2 rounded-md bg-blue-500/10 p-3 text-sm text-blue-500">
            Refreshing data in background...
          </div>
        )}
      </header>

      <main className="px-6 py-6 space-y-6">
        <div className="flex items-center gap-4">
          <select
            value={source}
            onChange={e => setSource(e.target.value as Source)}
            className="rounded-md border bg-background px-3 py-2 text-sm"
          >
            {SOURCES.map(s => (
              <option key={s} value={s}>
                {s === 'all' ? 'All Sources' : s.charAt(0).toUpperCase() + s.slice(1)}
              </option>
            ))}
          </select>

          <div className="flex rounded-md border">
            {TABS.map(t => (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className={`px-4 py-2 text-sm font-medium ${
                  tab === t.key
                    ? 'bg-primary text-primary-foreground'
                    : 'hover:bg-muted'
                }`}
              >
                {t.label}
              </button>
            ))}
          </div>
        </div>

        {totals && (
          <div className="grid grid-cols-4 gap-4">
            <div className="rounded-lg border p-4">
              <div className="text-sm text-muted-foreground">Total Cost</div>
              <div className="text-2xl font-bold">{totals.costFormatted ?? '$0.00'}</div>
            </div>
            <div className="rounded-lg border p-4">
              <div className="text-sm text-muted-foreground">Total Tokens</div>
              <div className="text-2xl font-bold">{(totals.totalTokens ?? 0).toLocaleString()}</div>
            </div>
            <div className="rounded-lg border p-4">
              <div className="text-sm text-muted-foreground">Input Tokens</div>
              <div className="text-2xl font-bold">{(totals.inputTokens ?? 0).toLocaleString()}</div>
            </div>
            <div className="rounded-lg border p-4">
              <div className="text-sm text-muted-foreground">Output Tokens</div>
              <div className="text-2xl font-bold">{(totals.outputTokens ?? 0).toLocaleString()}</div>
            </div>
          </div>
        )}

        <div className="rounded-lg border p-4">
          <h2 className="text-lg font-semibold mb-4">Usage Mosaic</h2>
          <ContributionCalendar
            data={calendarData as any}
            onDayClick={setSelectedDate}
          />
          {selectedDate && (
            <div className="mt-2 text-sm text-muted-foreground">
              Selected: {selectedDate}
            </div>
          )}
        </div>

        <div className="rounded-lg border p-4">
          <h2 className="text-lg font-semibold mb-4">
            {TABS.find(t => t.key === tab)?.label} Report
          </h2>
          <UsageTable data={tableData} type={tab} />
        </div>

        {snapshot?.status === 'nodata' && !loading && (
          <div className="text-center py-12 text-muted-foreground">
            No data available. Click Refresh to fetch usage data.
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
