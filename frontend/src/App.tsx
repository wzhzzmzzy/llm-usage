import { useState, useEffect, useCallback, useRef } from 'react';
import { HttpUsageApi } from './api/http-usage-api';
import type { Snapshot, Source, ReportType, UsageApi, RefreshStatus } from './api/types';
import { ContributionCalendar } from './components/contribution-calendar';
import { UsageTable } from './components/usage-table';
import { UsageChart } from './components/usage-chart';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { cn } from '@/lib/utils';
import { Check, ChevronsUpDown, RefreshCw, BarChart3, Table, Layers, Users } from 'lucide-react';

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

const SOURCES: { value: Source; label: string }[] = [
  { value: 'all', label: 'All Sources' },
  { value: 'claude', label: 'Claude' },
  { value: 'codex', label: 'Codex' },
  { value: 'gemini', label: 'Gemini' },
  { value: 'opencode', label: 'OpenCode' },
];

const REPORT_TABS: { key: ReportType; label: string }[] = [
  { key: 'daily', label: 'Daily' },
  { key: 'monthly', label: 'Monthly' },
  { key: 'session', label: 'Sessions' },
  { key: 'blocks', label: 'Blocks' },
];

function formatNumber(n: number | undefined): string {
  return (n ?? 0).toLocaleString();
}

function SourceCombobox({
  value,
  onValueChange,
}: {
  value: Source;
  onValueChange: (v: Source) => void;
}) {
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        className="inline-flex items-center justify-between rounded-md border bg-background px-3 py-2 text-sm hover:bg-muted"
        onClick={() => setOpen(!open)}
      >
        {SOURCES.find((s) => s.value === value)?.label ?? 'Select source'}
        <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
      </PopoverTrigger>
      <PopoverContent className="w-[180px] p-0">
        <Command>
          <CommandInput placeholder="Search source..." />
          <CommandList>
            <CommandEmpty>No source found.</CommandEmpty>
            <CommandGroup>
              {SOURCES.map((s) => (
                <CommandItem
                  key={s.value}
                  value={s.value}
                  onSelect={(current) => {
                    onValueChange(current as Source);
                    setOpen(false);
                  }}
                >
                  <Check
                    className={cn(
                      'mr-2 h-4 w-4',
                      value === s.value ? 'opacity-100' : 'opacity-0'
                    )}
                  />
                  {s.label}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}

function MetricCard({
  title,
  value,
}: {
  title: string;
  value: number | undefined;
}) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{formatNumber(value)}</div>
      </CardContent>
    </Card>
  );
}

function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [refreshStatus, setRefreshStatus] = useState<RefreshStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [source, setSource] = useState<Source>('all');
  const [tab, setTab] = useState<ReportType>('daily');
  const [view, setView] = useState<'table' | 'chart'>('table');
  const [segmentMode, setSegmentMode] = useState<'token-type' | 'agent-source'>('token-type');
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
        pollIntervalRef.current = window.setInterval(pollRefreshStatus, 5000);
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

  const sourceKey = source === 'all' ? 'all' : source;
  const dailyData = snapshot?.daily?.[`${sourceKey}_daily`];
  const monthlyData = snapshot?.monthly?.[`${sourceKey}_monthly`];
  const sessionData = snapshot?.session?.[`${sourceKey}_session`];
  const blocksData = snapshot?.blocks?.[`${sourceKey}_blocks`];

  const tableData =
    tab === 'daily' ? dailyData?.days ?? []
    : tab === 'monthly' ? monthlyData?.months ?? []
    : tab === 'session' ? sessionData?.sessions ?? []
    : blocksData?.blocks ?? [];

  const totals =
    tab === 'daily' ? dailyData?.totals
    : tab === 'monthly' ? monthlyData?.totals
    : tab === 'session' ? sessionData?.totals
    : blocksData?.totals;

  const calendarData =
    source === 'all'
      ? snapshot?.daily ?? {}
      : { [`${source}_daily`]: snapshot?.daily?.[`${source}_daily`] };

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b px-4 sm:px-6 py-4">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <h1 className="text-2xl font-semibold">LLM Usage Dashboard</h1>
          <div className="flex items-center gap-4">
            {snapshot?.lastSuccess && (
              <span className="text-sm text-muted-foreground hidden sm:inline">
                Updated: {new Date(snapshot.lastSuccess).toLocaleString()}
              </span>
            )}
            <Button onClick={handleRefresh} disabled={loading} size="sm">
              <RefreshCw className={cn('mr-2 h-4 w-4', loading && 'animate-spin')} />
              {loading ? 'Refreshing...' : 'Refresh'}
            </Button>
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

      <main className="px-4 sm:px-6 py-6 space-y-6">
        <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
          <SourceCombobox value={source} onValueChange={setSource} />

          <Tabs
            value={tab}
            onValueChange={(v) => setTab(v as ReportType)}
          >
            <TabsList>
              {REPORT_TABS.map((t) => (
                <TabsTrigger key={t.key} value={t.key}>
                  {t.label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          <div className="flex items-center gap-2 ml-auto">
            {view === 'chart' && (
              <div className="flex items-center gap-1 border rounded-md p-1">
                <Button
                  variant={segmentMode === 'token-type' ? 'default' : 'ghost'}
                  size="sm"
                  onClick={() => setSegmentMode('token-type')}
                  className="h-7 px-2 text-xs"
                >
                  <Layers className="h-3 w-3 mr-1" />
                  Token Type
                </Button>
                <Button
                  variant={segmentMode === 'agent-source' ? 'default' : 'ghost'}
                  size="sm"
                  onClick={() => setSegmentMode('agent-source')}
                  className="h-7 px-2 text-xs"
                  disabled={source !== 'all'}
                >
                  <Users className="h-3 w-3 mr-1" />
                  Agent Source
                </Button>
              </div>
            )}
            <div className="flex items-center gap-1">
              <Button
                variant={view === 'table' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setView('table')}
              >
                <Table className="h-4 w-4" />
              </Button>
              <Button
                variant={view === 'chart' ? 'default' : 'outline'}
                size="sm"
                onClick={() => setView('chart')}
              >
                <BarChart3 className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>

        {totals && (
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
            <MetricCard title="Total Tokens" value={totals.totalTokens} />
            <MetricCard title="Input" value={totals.inputTokens} />
            <MetricCard title="Cache Hit" value={totals.cacheReadTokens} />
            <MetricCard title="Output" value={totals.outputTokens} />
          </div>
        )}

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Usage Mosaic</CardTitle>
              {selectedDate && (
                <Badge variant="secondary">{selectedDate}</Badge>
              )}
            </div>
          </CardHeader>
          <CardContent>
            <ContributionCalendar
              data={calendarData as any}
              onDayClick={setSelectedDate}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>
              {REPORT_TABS.find((t) => t.key === tab)?.label} Report
            </CardTitle>
          </CardHeader>
          <CardContent>
            {view === 'table' ? (
              <UsageTable data={tableData} type={tab} snapshot={snapshot ?? undefined} source={source} />
            ) : (
              <UsageChart
                data={tableData}
                type={tab}
                snapshot={snapshot ?? undefined}
                segmentMode={source === 'all' ? segmentMode : 'token-type'}
              />
            )}
          </CardContent>
        </Card>

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
