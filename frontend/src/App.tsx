import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { HttpUsageApi } from './api/http-usage-api';
import type { Snapshot, Source, ReportType, UsageApi, RefreshStatus, PricingMap, ModelPricing } from './api/types';
import { ContributionCalendar } from './components/contribution-calendar';
import { UsageTable } from './components/usage-table';
import { UsageChart } from './components/usage-chart';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
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
import { Check, ChevronsUpDown, RefreshCw, BarChart3, Table, Layers, Users, Info } from 'lucide-react';
import { formatTokens } from '@/components/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useTranslation } from './i18n/context';
import { makeRange, dayCount } from '@/lib/selection';
import type { Selection } from '@/lib/selection';

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
      async getPricing() {
        const invoke = await getTauriInvoke();
        if (!invoke) throw new Error('Tauri not available');
        return invoke('get_pricing');
      },
    };
  }
  return new HttpUsageApi();
}

const api = createApi();

function formatCost(n: number | undefined): string {
  if (n === undefined || n === 0) return '$0.00';
  if (n < 0.01) return `$${n.toFixed(4)}`;
  if (n < 1) return `$${n.toFixed(3)}`;
  return `$${n.toFixed(2)}`;
}

function estimateCost(
  inputTokens: number,
  outputTokens: number,
  reasoningTokens: number,
  cacheReadTokens: number,
  modelBreakdown: Array<{ model: string; inputTokens: number; outputTokens: number; reasoningTokens: number; cacheReadTokens: number }> | undefined,
  pricing: PricingMap | null
): { cost: number; matched: boolean } {
  const defaultPricing: ModelPricing = { input: 3e-6, output: 15e-6, cacheCreate: 3.75e-6, cacheRead: 0.3e-6 };

  const findPricing = (model: string): { pricing: ModelPricing; matched: boolean } => {
    if (!pricing) return { pricing: defaultPricing, matched: false };
    if (pricing[model]) return { pricing: pricing[model], matched: true };
    const normalized = model.replace(/[.@]/g, '-');
    for (const [key, value] of Object.entries(pricing)) {
      if (key.includes(model) || model.includes(key) || key.includes(normalized) || normalized.includes(key)) {
        return { pricing: value, matched: true };
      }
    }
    return { pricing: defaultPricing, matched: false };
  };

  if (modelBreakdown && modelBreakdown.length > 0) {
    let anyMatched = false;
    const cost = modelBreakdown.reduce((total, item) => {
      const { pricing: p, matched } = findPricing(item.model);
      if (matched) anyMatched = true;
      return total
        + item.inputTokens * p.input
        + (item.outputTokens + item.reasoningTokens) * p.output
        + item.cacheReadTokens * p.cacheRead;
    }, 0);
    return { cost, matched: anyMatched };
  }

  const p = defaultPricing;
  return {
    cost: inputTokens * p.input + (outputTokens + reasoningTokens) * p.output + cacheReadTokens * p.cacheRead,
    matched: false
  };
}

function SourceCombobox({
  value,
  onValueChange,
  sources,
}: {
  value: Source;
  onValueChange: (v: Source) => void;
  sources: { value: Source; label: string }[];
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        className="inline-flex items-center justify-between rounded-md border bg-background px-3 py-2 text-sm hover:bg-muted"
        onClick={() => setOpen(!open)}
      >
        {sources.find((s) => s.value === value)?.label ?? t('source.placeholder')}
        <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
      </PopoverTrigger>
      <PopoverContent className="w-[180px] p-0">
        <Command>
          <CommandInput placeholder={t('source.search')} />
          <CommandList>
            <CommandEmpty>{t('source.notFound')}</CommandEmpty>
            <CommandGroup>
              {sources.map((s) => (
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
  total,
  tooltip,
}: {
  title: string;
  value: number | undefined;
  total: number | undefined;
  tooltip?: React.ReactNode;
}) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-1">
          {title}
          {tooltip}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{formatTokens(value)}</div>
        <div className="text-xs text-muted-foreground mt-0.5">
          / {formatTokens(total)}
        </div>
      </CardContent>
    </Card>
  );
}

function App() {
  const { t } = useTranslation();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [pricing, setPricing] = useState<PricingMap | null>(null);
  const [loading, setLoading] = useState(false);
  const [refreshStatus, setRefreshStatus] = useState<RefreshStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [source, setSource] = useState<Source>('all');
  const [tab, setTab] = useState<ReportType>('daily');
  const [view, setView] = useState<'table' | 'chart'>('table');
  const [segmentMode, setSegmentMode] = useState<'token-type' | 'agent-source'>('token-type');
  const [selection, setSelection] = useState<Selection>({ type: 'none' });
  const hasRefreshed = useRef(false);
  const pollIntervalRef = useRef<number | null>(null);

  const handleDayClick = useCallback((date: string) => {
    setSelection((prev) => {
      if (prev.type === 'none' || prev.type === 'range') {
        return { type: 'pending', start: date };
      }
      // pending → confirm range (auto-sorted)
      return makeRange(prev.start, date);
    });
  }, []);

  const SOURCES: { value: Source; label: string }[] = [
    { value: 'all', label: t('source.all') },
    { value: 'claude', label: 'Claude' },
    { value: 'codex', label: 'Codex' },
    { value: 'gemini', label: 'Gemini' },
    { value: 'opencode', label: 'OpenCode' },
  ];

  const REPORT_TABS: { key: ReportType; label: string }[] = [
    { key: 'daily', label: t('tab.daily') },
    { key: 'monthly', label: t('tab.monthly') },
    { key: 'session', label: t('tab.session') },
    { key: 'blocks', label: t('tab.blocks') },
  ];

  const loadSnapshot = useCallback(async () => {
    try {
      const [snap, priceData] = await Promise.all([
        api.getSnapshot(),
        api.getPricing().catch(() => null),
      ]);
      setSnapshot(snap);
      if (priceData) setPricing(priceData);
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

  const today = new Date().toLocaleDateString('en-CA');

  const allDailyData = snapshot?.daily?.[`${sourceKey}_daily`];
  const dailyTotals = allDailyData?.totals;

  const { activeTotals, activeModelBreakdown } = useMemo(() => {
    if (selection.type === 'range') {
      const inRange = allDailyData?.days.filter(
        (d) => d.date >= selection.start && d.date <= selection.end,
      ) ?? [];

      const totals = {
        totalTokens: inRange.reduce((s, r) => s + (r.totalTokens ?? 0), 0),
        inputTokens: inRange.reduce((s, r) => s + (r.inputTokens ?? 0), 0),
        cacheReadTokens: inRange.reduce((s, r) => s + (r.cacheReadTokens ?? 0), 0),
        outputTokens: inRange.reduce((s, r) => s + (r.outputTokens ?? 0), 0),
        reasoningTokens: inRange.reduce((s, r) => s + (r.reasoningTokens ?? 0), 0),
      };

      const breakdown: Array<{
        model: string;
        inputTokens: number;
        outputTokens: number;
        reasoningTokens: number;
        cacheReadTokens: number;
      }> = [];
      for (const row of inRange) {
        for (const item of row.modelBreakdown ?? []) {
          const existing = breakdown.find((a) => a.model === item.model);
          if (existing) {
            existing.inputTokens += item.inputTokens;
            existing.outputTokens += item.outputTokens;
            existing.reasoningTokens += item.reasoningTokens;
            existing.cacheReadTokens += item.cacheReadTokens;
          } else {
            breakdown.push({ ...item });
          }
        }
      }

      return { activeTotals: totals, activeModelBreakdown: breakdown };
    }

    // pending or none: use single day
    const activeDate = selection.type === 'pending' ? selection.start : today;
    const row = allDailyData?.days.find((d) => d.date === activeDate);
    return {
      activeTotals: {
        totalTokens: row?.totalTokens ?? 0,
        inputTokens: row?.inputTokens ?? 0,
        cacheReadTokens: row?.cacheReadTokens ?? 0,
        outputTokens: row?.outputTokens ?? 0,
        reasoningTokens: row?.reasoningTokens ?? 0,
      },
      activeModelBreakdown: row?.modelBreakdown ?? [],
    };
  }, [selection, allDailyData, today]);

  const allDailyAggregatedBreakdown: Array<{
    model: string;
    inputTokens: number;
    outputTokens: number;
    reasoningTokens: number;
    cacheReadTokens: number;
  }> = [];
  for (const row of allDailyData?.days ?? []) {
    if (!row.modelBreakdown) continue;
    for (const item of row.modelBreakdown) {
      const existing = allDailyAggregatedBreakdown.find((a) => a.model === item.model);
      if (existing) {
        existing.inputTokens += item.inputTokens;
        existing.outputTokens += item.outputTokens;
        existing.reasoningTokens += item.reasoningTokens;
        existing.cacheReadTokens += item.cacheReadTokens;
      } else {
        allDailyAggregatedBreakdown.push({
          model: item.model,
          inputTokens: item.inputTokens,
          outputTokens: item.outputTokens,
          reasoningTokens: item.reasoningTokens,
          cacheReadTokens: item.cacheReadTokens,
        });
      }
    }
  }

  const cacheHitRate =
    activeTotals.totalTokens > 0
      ? ((activeTotals.cacheReadTokens / activeTotals.totalTokens) * 100).toFixed(1)
      : '0.0';

  const cacheHitTooltip = (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger className="text-muted-foreground/60 hover:text-muted-foreground cursor-default">
          <Info className="h-3.5 w-3.5" />
        </TooltipTrigger>
        <TooltipContent>{t('tooltip.cacheHit', { rate: cacheHitRate })}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );

  const tableData =
    tab === 'daily' ? dailyData?.days ?? []
    : tab === 'monthly' ? monthlyData?.months ?? []
    : tab === 'session' ? sessionData?.sessions ?? []
    : blocksData?.blocks ?? [];

  const aggregatedModelBreakdown: Array<{ model: string; inputTokens: number; outputTokens: number; reasoningTokens: number; cacheReadTokens: number }> = [];
  for (const row of tableData) {
    const breakdown = 'modelBreakdown' in row ? row.modelBreakdown : undefined;
    if (!breakdown) continue;
    for (const item of breakdown) {
      const existing = aggregatedModelBreakdown.find(a => a.model === item.model);
      if (existing) {
        existing.inputTokens += item.inputTokens;
        existing.outputTokens += item.outputTokens;
        existing.reasoningTokens += item.reasoningTokens;
        existing.cacheReadTokens += item.cacheReadTokens;
      } else {
        aggregatedModelBreakdown.push({
          model: item.model,
          inputTokens: item.inputTokens,
          outputTokens: item.outputTokens,
          reasoningTokens: item.reasoningTokens,
          cacheReadTokens: item.cacheReadTokens,
        });
      }
    }
  }

  const calendarData =
    source === 'all'
      ? { all: snapshot?.daily?.['all_daily'] }
      : { [`${source}_daily`]: snapshot?.daily?.[`${source}_daily`] };

  const currentTabLabel = REPORT_TABS.find((r) => r.key === tab)?.label ?? tab;

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b px-4 sm:px-6 py-4">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <h1 className="text-2xl font-semibold">{t('app.title')}</h1>
          <div className="flex items-center gap-4">
            {snapshot?.lastSuccess && (
              <span className="text-sm text-muted-foreground hidden sm:inline">
                {t('header.updated')}{new Date(snapshot.lastSuccess).toLocaleString()}
              </span>
            )}
            <Button onClick={handleRefresh} disabled={loading} size="sm">
              <RefreshCw className={cn('mr-2 h-4 w-4', loading && 'animate-spin')} />
              {loading ? t('btn.refreshing') : t('btn.refresh')}
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
            {t('status.refreshing')}
          </div>
        )}
      </header>

      <main className="px-4 sm:px-6 py-6 space-y-6">
        <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
          <SourceCombobox value={source} onValueChange={setSource} sources={SOURCES} />

          <Tabs
            value={tab}
            onValueChange={(v) => setTab(v as ReportType)}
          >
            <TabsList>
              {REPORT_TABS.map((r) => (
                <TabsTrigger key={r.key} value={r.key}>
                  {r.label}
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
                  {t('chart.tokenType')}
                </Button>
                <Button
                  variant={segmentMode === 'agent-source' ? 'default' : 'ghost'}
                  size="sm"
                  onClick={() => setSegmentMode('agent-source')}
                  className="h-7 px-2 text-xs"
                  disabled={source !== 'all'}
                >
                  <Users className="h-3 w-3 mr-1" />
                  {t('chart.agentSource')}
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

        {(activeTotals || dailyTotals) && (
          <div className="space-y-2">
            {selection.type !== 'none' && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>
                  {selection.type === 'pending'
                    ? t('mosaic.viewing', { date: selection.start })
                    : t('mosaic.range', {
                        start: selection.start,
                        end: selection.end,
                        days: dayCount(selection.start, selection.end),
                      })}
                </span>
                <button
                  onClick={() => setSelection({ type: 'none' })}
                  className="rounded-sm opacity-70 hover:opacity-100"
                  aria-label={t('btn.backToday')}
                >
                  ×
                </button>
              </div>
            )}
            <div className="grid grid-cols-2 sm:grid-cols-6 gap-4">
              <MetricCard
                title={t('metric.totalTokens')}
                value={activeTotals.totalTokens}
                total={dailyTotals?.totalTokens}
              />
              <MetricCard
                title={t('metric.input')}
                value={activeTotals.inputTokens}
                total={dailyTotals?.inputTokens}
              />
              <MetricCard
                title={t('metric.cacheHit')}
                value={activeTotals.cacheReadTokens}
                total={dailyTotals?.cacheReadTokens}
                tooltip={cacheHitTooltip}
              />
              <MetricCard
                title={t('metric.output')}
                value={activeTotals.outputTokens}
                total={dailyTotals?.outputTokens}
              />
              <MetricCard
                title={t('metric.reasoning')}
                value={activeTotals.reasoningTokens}
                total={dailyTotals?.reasoningTokens}
              />
              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-medium text-muted-foreground">
                    {t('metric.cost')}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {(() => {
                    const activeCost = estimateCost(
                      activeTotals.inputTokens,
                      activeTotals.outputTokens,
                      activeTotals.reasoningTokens,
                      activeTotals.cacheReadTokens,
                      activeModelBreakdown,
                      pricing
                    );
                    const totalCost = estimateCost(
                      dailyTotals?.inputTokens ?? 0,
                      dailyTotals?.outputTokens ?? 0,
                      dailyTotals?.reasoningTokens ?? 0,
                      dailyTotals?.cacheReadTokens ?? 0,
                      allDailyAggregatedBreakdown,
                      pricing
                    );
                    return (
                      <>
                        <div className="text-2xl font-bold">
                          {formatCost(activeCost.cost)}
                          {!activeCost.matched && pricing && (
                            <span className="text-xs text-muted-foreground ml-2">{t('metric.costDefault')}</span>
                          )}
                        </div>
                        <div className="text-xs text-muted-foreground mt-0.5">
                          / {formatCost(totalCost.cost)}
                        </div>
                      </>
                    );
                  })()}
                </CardContent>
              </Card>
            </div>
          </div>
        )}

        <Card>
          <CardHeader>
            <CardTitle>{t('mosaic.title')}</CardTitle>
          </CardHeader>
          <CardContent>
            <ContributionCalendar
              data={calendarData as any}
              selection={selection}
              onDayClick={handleDayClick}
              pricing={pricing}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>
              {t('report.title', { tab: currentTabLabel })}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {view === 'table' ? (
              <UsageTable data={tableData} type={tab} snapshot={snapshot ?? undefined} source={source} pricing={pricing} />
            ) : (
              <UsageChart
                data={tableData}
                type={tab}
                snapshot={snapshot ?? undefined}
                segmentMode={source === 'all' ? segmentMode : 'token-type'}
                pricing={pricing}
              />
            )}
          </CardContent>
        </Card>

        {snapshot?.status === 'nodata' && !loading && (
          <div className="text-center py-12 text-muted-foreground">
            {t('status.noData')}
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
