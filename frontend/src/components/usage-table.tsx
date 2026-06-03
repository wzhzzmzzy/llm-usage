import { useState } from 'react';
import type { DailyRow, MonthlyRow, SessionRow, BlockRow, ModelBreakdown, Snapshot, PricingMap, ModelPricing } from '../api/types';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { ChevronDown, Layers, Cpu } from 'lucide-react';

interface UsageTableProps {
  data: DailyRow[] | MonthlyRow[] | SessionRow[] | BlockRow[];
  type: 'daily' | 'monthly' | 'session' | 'blocks';
  snapshot?: Snapshot;
  source?: string;
  pricing?: PricingMap | null;
}

function fmt(n: number | undefined): string {
  return (n ?? 0).toLocaleString();
}

function fmtCost(n: number): string {
  if (n === 0) return '-';
  if (n < 0.01) return `$${n.toFixed(4)}`;
  if (n < 1) return `$${n.toFixed(3)}`;
  return `$${n.toFixed(2)}`;
}

const DEFAULT_PRICING: ModelPricing = { input: 3e-6, output: 15e-6, cacheCreate: 3.75e-6, cacheRead: 0.3e-6 };

function findModelPricing(model: string, pricing: PricingMap | null | undefined): ModelPricing {
  if (!pricing) return DEFAULT_PRICING;
  if (pricing[model]) return pricing[model];
  const normalized = model.replace(/[.@]/g, '-');
  for (const [key, value] of Object.entries(pricing)) {
    if (key.includes(model) || model.includes(key) || key.includes(normalized) || normalized.includes(key)) {
      return value;
    }
  }
  return DEFAULT_PRICING;
}

function estimateRowCost(
  inputTokens: number,
  outputTokens: number,
  cacheReadTokens: number,
  modelBreakdown?: ModelBreakdown[] | null,
  pricing?: PricingMap | null,
): number {
  if (modelBreakdown && modelBreakdown.length > 0) {
    return modelBreakdown.reduce((total, mb) => {
      const p = findModelPricing(mb.model, pricing);
      return total + mb.inputTokens * p.input + mb.outputTokens * p.output + mb.cacheReadTokens * p.cacheRead;
    }, 0);
  }
  const p = DEFAULT_PRICING;
  return inputTokens * p.input + outputTokens * p.output + cacheReadTokens * p.cacheRead;
}

interface ExpandedState {
  [key: string]: 'source' | 'model' | null;
}

function ExpandButton({
  isExpanded,
  onClick,
  type,
}: {
  isExpanded: boolean;
  onClick: () => void;
  type: 'source' | 'model';
}) {
  const Icon = type === 'source' ? Layers : Cpu;
  const expandedIcon = <ChevronDown className="h-3.5 w-3.5" />;
  const collapsedIcon = <Icon className="h-3.5 w-3.5" />;

  return (
    <button
      onClick={onClick}
      className={`p-0.5 hover:bg-muted rounded ${type === 'source' ? 'text-emerald-600' : 'text-violet-600'}`}
      title={type === 'source' ? 'Drill down by source' : 'Drill down by model'}
    >
      {isExpanded ? expandedIcon : collapsedIcon}
    </button>
  );
}

function SourceBreakdownRow({
  sourceKey,
  date,
  snapshot,
  type,
  pricing,
}: {
  sourceKey: string;
  date: string;
  snapshot: Snapshot;
  type: 'daily' | 'monthly';
  pricing?: PricingMap | null;
}) {
  const sources = ['claude', 'codex', 'gemini', 'opencode'] as const;

  return (
    <>
      {sources.map((src) => {
        const reportKey = `${src}_${type}`;
        const report = type === 'daily' ? snapshot.daily?.[reportKey] : snapshot.monthly?.[reportKey];
        if (!report) return null;

        const rows = type === 'daily'
          ? (report as any).days?.filter((r: any) => r.date === date)
          : (report as any).months?.filter((r: any) => r.month === date);

        if (!rows || rows.length === 0) return null;

        const row = rows[0];
        const cost = estimateRowCost(row.inputTokens, row.outputTokens, row.cacheReadTokens, row.modelBreakdown, pricing);
        return (
          <TableRow key={`${sourceKey}-${src}`} className="bg-muted/30">
            <TableCell className="pl-8 text-muted-foreground">
              <span className="inline-flex items-center gap-1">
                <span className="w-2 h-2 rounded-full bg-primary/50" />
                {src}
              </span>
            </TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(row.totalTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(row.inputTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(row.cacheReadTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(row.outputTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmtCost(cost)}</TableCell>
            <TableCell className="text-muted-foreground">
              {row.modelsUsed?.join(', ') ?? '-'}
            </TableCell>
          </TableRow>
        );
      })}
    </>
  );
}

function ModelBreakdownRows({ breakdown, pricing }: { breakdown: ModelBreakdown[]; pricing?: PricingMap | null }) {
  return (
    <>
      {breakdown.map((mb) => {
        const p = findModelPricing(mb.model, pricing);
        const cost = mb.inputTokens * p.input + mb.outputTokens * p.output + mb.cacheReadTokens * p.cacheRead;
        return (
          <TableRow key={mb.model} className="bg-muted/30">
            <TableCell className="pl-8 text-muted-foreground">
              <span className="inline-flex items-center gap-1">
                <span className="w-2 h-2 rounded-full bg-blue-500/50" />
                {mb.model}
              </span>
            </TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(mb.totalTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(mb.inputTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(mb.cacheReadTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmt(mb.outputTokens)}</TableCell>
            <TableCell className="text-right text-muted-foreground">{fmtCost(cost)}</TableCell>
            <TableCell className="text-muted-foreground">
              {fmt(mb.requestCount)} reqs
            </TableCell>
          </TableRow>
        );
      })}
    </>
  );
}

function DailyTable({
  data,
  snapshot,
  source,
  pricing,
}: {
  data: DailyRow[];
  snapshot?: Snapshot;
  source?: string;
  pricing?: PricingMap | null;
}) {
  const [expanded, setExpanded] = useState<ExpandedState>({});

  const toggleExpand = (key: string, type: 'source' | 'model') => {
    setExpanded((prev) => ({
      ...prev,
      [key]: prev[key] === type ? null : type,
    }));
  };

  const showSourceDrilldown = source === 'all' && snapshot;

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Date</TableHead>
          <TableHead className="text-right">Total</TableHead>
          <TableHead className="text-right">Input</TableHead>
          <TableHead className="text-right">Cache Hit</TableHead>
          <TableHead className="text-right">Output</TableHead>
          <TableHead className="text-right">Cost</TableHead>
          <TableHead>Models</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.length === 0 ? (
          <TableRow>
            <TableCell colSpan={7} className="h-24 text-center text-muted-foreground">
              No data available
            </TableCell>
          </TableRow>
        ) : (
          data.map((row) => {
            const rowKey = row.date;
            const isExpanded = expanded[rowKey];
            const hasModelBreakdown = row.modelBreakdown && row.modelBreakdown.length > 0;
            const cost = estimateRowCost(row.inputTokens, row.outputTokens, row.cacheReadTokens, row.modelBreakdown, pricing);

            return (
              <>
                <TableRow
                  key={rowKey}
                  className={isExpanded ? 'bg-muted/50' : ''}
                >
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-1">
                      {showSourceDrilldown && (
                        <ExpandButton
                          isExpanded={isExpanded === 'source'}
                          onClick={() => toggleExpand(rowKey, 'source')}
                          type="source"
                        />
                      )}
                      {hasModelBreakdown && (
                        <ExpandButton
                          isExpanded={isExpanded === 'model'}
                          onClick={() => toggleExpand(rowKey, 'model')}
                          type="model"
                        />
                      )}
                      {!showSourceDrilldown && !hasModelBreakdown && (
                        <span className="w-5" />
                      )}
                      {row.date}
                    </div>
                  </TableCell>
                  <TableCell className="text-right">{fmt(row.totalTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.inputTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.cacheReadTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.outputTokens)}</TableCell>
                  <TableCell className="text-right">{fmtCost(cost)}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {row.modelsUsed?.join(', ') ?? '-'}
                  </TableCell>
                </TableRow>
                {isExpanded === 'source' && showSourceDrilldown && (
                  <SourceBreakdownRow
                    sourceKey={rowKey}
                    date={row.date}
                    snapshot={snapshot}
                    type="daily"
                    pricing={pricing}
                  />
                )}
                {isExpanded === 'model' && hasModelBreakdown && (
                  <ModelBreakdownRows breakdown={row.modelBreakdown!} pricing={pricing} />
                )}
              </>
            );
          })
        )}
      </TableBody>
    </Table>
  );
}

function MonthlyTable({
  data,
  snapshot,
  source,
  pricing,
}: {
  data: MonthlyRow[];
  snapshot?: Snapshot;
  source?: string;
  pricing?: PricingMap | null;
}) {
  const [expanded, setExpanded] = useState<ExpandedState>({});

  const toggleExpand = (key: string, type: 'source' | 'model') => {
    setExpanded((prev) => ({
      ...prev,
      [key]: prev[key] === type ? null : type,
    }));
  };

  const showSourceDrilldown = source === 'all' && snapshot;

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Month</TableHead>
          <TableHead className="text-right">Total</TableHead>
          <TableHead className="text-right">Input</TableHead>
          <TableHead className="text-right">Cache Hit</TableHead>
          <TableHead className="text-right">Output</TableHead>
          <TableHead className="text-right">Cost</TableHead>
          <TableHead>Models</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.length === 0 ? (
          <TableRow>
            <TableCell colSpan={7} className="h-24 text-center text-muted-foreground">
              No data available
            </TableCell>
          </TableRow>
        ) : (
          data.map((row) => {
            const rowKey = row.month;
            const isExpanded = expanded[rowKey];
            const hasModelBreakdown = row.modelBreakdown && row.modelBreakdown.length > 0;
            const cost = estimateRowCost(row.inputTokens, row.outputTokens, row.cacheReadTokens, row.modelBreakdown, pricing);

            return (
              <>
                <TableRow
                  key={rowKey}
                  className={isExpanded ? 'bg-muted/50' : ''}
                >
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-1">
                      {showSourceDrilldown && (
                        <ExpandButton
                          isExpanded={isExpanded === 'source'}
                          onClick={() => toggleExpand(rowKey, 'source')}
                          type="source"
                        />
                      )}
                      {hasModelBreakdown && (
                        <ExpandButton
                          isExpanded={isExpanded === 'model'}
                          onClick={() => toggleExpand(rowKey, 'model')}
                          type="model"
                        />
                      )}
                      {!showSourceDrilldown && !hasModelBreakdown && (
                        <span className="w-5" />
                      )}
                      {row.month}
                    </div>
                  </TableCell>
                  <TableCell className="text-right">{fmt(row.totalTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.inputTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.cacheReadTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.outputTokens)}</TableCell>
                  <TableCell className="text-right">{fmtCost(cost)}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {row.modelsUsed?.join(', ') ?? '-'}
                  </TableCell>
                </TableRow>
                {isExpanded === 'source' && showSourceDrilldown && (
                  <SourceBreakdownRow
                    sourceKey={rowKey}
                    date={row.month}
                    snapshot={snapshot}
                    type="monthly"
                    pricing={pricing}
                  />
                )}
                {isExpanded === 'model' && hasModelBreakdown && (
                  <ModelBreakdownRows breakdown={row.modelBreakdown!} pricing={pricing} />
                )}
              </>
            );
          })
        )}
      </TableBody>
    </Table>
  );
}

function SessionTable({ data, pricing }: { data: SessionRow[]; pricing?: PricingMap | null }) {
  const [expanded, setExpanded] = useState<ExpandedState>({});

  const toggleExpand = (key: string) => {
    setExpanded((prev) => ({
      ...prev,
      [key]: prev[key] === 'model' ? null : 'model',
    }));
  };

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Session</TableHead>
          <TableHead>Project</TableHead>
          <TableHead className="text-right">Total</TableHead>
          <TableHead className="text-right">Input</TableHead>
          <TableHead className="text-right">Cache Hit</TableHead>
          <TableHead className="text-right">Output</TableHead>
          <TableHead className="text-right">Cost</TableHead>
          <TableHead>Last Active</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.length === 0 ? (
          <TableRow>
            <TableCell colSpan={8} className="h-24 text-center text-muted-foreground">
              No data available
            </TableCell>
          </TableRow>
        ) : (
          data.map((row) => {
            const rowKey = row.sessionId;
            const isExpanded = expanded[rowKey];
            const hasModelBreakdown = row.modelBreakdown && row.modelBreakdown.length > 0;
            const cost = estimateRowCost(row.inputTokens, row.outputTokens, row.cacheReadTokens, row.modelBreakdown, pricing);

            return (
              <>
                <TableRow
                  key={rowKey}
                  className={isExpanded ? 'bg-muted/50' : ''}
                >
                  <TableCell className="font-mono text-xs">
                    <div className="flex items-center gap-1">
                      {hasModelBreakdown && (
                        <ExpandButton
                          isExpanded={!!isExpanded}
                          onClick={() => toggleExpand(rowKey)}
                          type="model"
                        />
                      )}
                      {!hasModelBreakdown && <span className="w-5" />}
                      {row.sessionId.slice(0, 8)}
                    </div>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {row.projectPath?.split('/').pop() ?? '-'}
                  </TableCell>
                  <TableCell className="text-right">{fmt(row.totalTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.inputTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.cacheReadTokens)}</TableCell>
                  <TableCell className="text-right">{fmt(row.outputTokens)}</TableCell>
                  <TableCell className="text-right">{fmtCost(cost)}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {row.lastActivity
                      ? new Date(row.lastActivity).toLocaleDateString()
                      : '-'}
                  </TableCell>
                </TableRow>
                {isExpanded && hasModelBreakdown && (
                  <ModelBreakdownRows breakdown={row.modelBreakdown!} pricing={pricing} />
                )}
              </>
            );
          })
        )}
      </TableBody>
    </Table>
  );
}

function BlockTable({ data, pricing }: { data: BlockRow[]; pricing?: PricingMap | null }) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Status</TableHead>
          <TableHead>Start Time</TableHead>
          <TableHead>End Time</TableHead>
          <TableHead className="text-right">Total</TableHead>
          <TableHead className="text-right">Input</TableHead>
          <TableHead className="text-right">Cache Hit</TableHead>
          <TableHead className="text-right">Output</TableHead>
          <TableHead className="text-right">Cost</TableHead>
          <TableHead>Models</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.length === 0 ? (
          <TableRow>
            <TableCell colSpan={9} className="h-24 text-center text-muted-foreground">
              No data available
            </TableCell>
          </TableRow>
        ) : (
          data.map((row) => {
            const cost = estimateRowCost(row.inputTokens, row.outputTokens, row.cacheReadTokens, null, pricing);
            return (
              <TableRow key={row.blockId} className={row.isActive ? 'bg-primary/5' : ''}>
                <TableCell>
                  {row.isActive ? (
                    <span className="inline-flex items-center rounded-full bg-green-500/10 px-2 py-0.5 text-xs font-medium text-green-600">
                      Active
                    </span>
                  ) : (
                    <span className="inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                      Done
                    </span>
                  )}
                </TableCell>
                <TableCell className="text-muted-foreground">
                  {formatBlockTime(row.startTime)}
                </TableCell>
                <TableCell className="text-muted-foreground">
                  {row.endTime ? formatBlockTime(row.endTime) : '-'}
                </TableCell>
                <TableCell className="text-right font-medium">{fmt(row.totalTokens)}</TableCell>
                <TableCell className="text-right">{fmt(row.inputTokens)}</TableCell>
                <TableCell className="text-right">{fmt(row.cacheReadTokens)}</TableCell>
                <TableCell className="text-right">{fmt(row.outputTokens)}</TableCell>
                <TableCell className="text-right">{fmtCost(cost)}</TableCell>
                <TableCell className="text-muted-foreground">
                  {row.modelsUsed?.join(', ') ?? '-'}
                </TableCell>
              </TableRow>
            );
          })
        )}
      </TableBody>
    </Table>
  );
}

function formatBlockTime(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function UsageTable({ data, type, snapshot, source, pricing }: UsageTableProps) {
  switch (type) {
    case 'daily':
      return <DailyTable data={data as DailyRow[]} snapshot={snapshot} source={source} pricing={pricing} />;
    case 'monthly':
      return <MonthlyTable data={data as MonthlyRow[]} snapshot={snapshot} source={source} pricing={pricing} />;
    case 'session':
      return <SessionTable data={data as SessionRow[]} pricing={pricing} />;
    case 'blocks':
      return <BlockTable data={data as BlockRow[]} pricing={pricing} />;
  }
}
