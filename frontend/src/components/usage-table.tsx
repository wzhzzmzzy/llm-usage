import { useState } from 'react';
import type { DailyRow, MonthlyRow, SessionRow, BlockRow, ModelBreakdown, Snapshot } from '../api/types';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { ChevronRight, ChevronDown } from 'lucide-react';

interface UsageTableProps {
  data: DailyRow[] | MonthlyRow[] | SessionRow[] | BlockRow[];
  type: 'daily' | 'monthly' | 'session' | 'blocks';
  snapshot?: Snapshot;
  source?: string;
}

function fmt(n: number | undefined): string {
  return (n ?? 0).toLocaleString();
}

interface ExpandedState {
  [key: string]: 'source' | 'model' | null;
}

function SourceBreakdownRow({
  sourceKey,
  date,
  snapshot,
  type,
}: {
  sourceKey: string;
  date: string;
  snapshot: Snapshot;
  type: 'daily' | 'monthly';
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
            <TableCell className="text-muted-foreground">
              {row.modelsUsed?.join(', ') ?? '-'}
            </TableCell>
          </TableRow>
        );
      })}
    </>
  );
}

function ModelBreakdownRows({ breakdown }: { breakdown: ModelBreakdown[] }) {
  return (
    <>
      {breakdown.map((mb) => (
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
          <TableCell className="text-muted-foreground">
            {fmt(mb.requestCount)} reqs
          </TableCell>
        </TableRow>
      ))}
    </>
  );
}

function DailyTable({
  data,
  snapshot,
  source,
}: {
  data: DailyRow[];
  snapshot?: Snapshot;
  source?: string;
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
          <TableHead>Models</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.length === 0 ? (
          <TableRow>
            <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
              No data available
            </TableCell>
          </TableRow>
        ) : (
          data.map((row) => {
            const rowKey = row.date;
            const isExpanded = expanded[rowKey];
            const hasModelBreakdown = row.modelBreakdown && row.modelBreakdown.length > 0;

            return (
              <>
                <TableRow
                  key={rowKey}
                  className={isExpanded ? 'bg-muted/50' : ''}
                >
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-1">
                      {showSourceDrilldown && (
                        <button
                          onClick={() => toggleExpand(rowKey, 'source')}
                          className="p-0.5 hover:bg-muted rounded"
                        >
                          {isExpanded === 'source' ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                        </button>
                      )}
                      {hasModelBreakdown && (
                        <button
                          onClick={() => toggleExpand(rowKey, 'model')}
                          className="p-0.5 hover:bg-muted rounded"
                        >
                          {isExpanded === 'model' ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                        </button>
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
                  />
                )}
                {isExpanded === 'model' && hasModelBreakdown && (
                  <ModelBreakdownRows breakdown={row.modelBreakdown!} />
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
}: {
  data: MonthlyRow[];
  snapshot?: Snapshot;
  source?: string;
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
          <TableHead>Models</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.length === 0 ? (
          <TableRow>
            <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
              No data available
            </TableCell>
          </TableRow>
        ) : (
          data.map((row) => {
            const rowKey = row.month;
            const isExpanded = expanded[rowKey];
            const hasModelBreakdown = row.modelBreakdown && row.modelBreakdown.length > 0;

            return (
              <>
                <TableRow
                  key={rowKey}
                  className={isExpanded ? 'bg-muted/50' : ''}
                >
                  <TableCell className="font-medium">
                    <div className="flex items-center gap-1">
                      {showSourceDrilldown && (
                        <button
                          onClick={() => toggleExpand(rowKey, 'source')}
                          className="p-0.5 hover:bg-muted rounded"
                        >
                          {isExpanded === 'source' ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                        </button>
                      )}
                      {hasModelBreakdown && (
                        <button
                          onClick={() => toggleExpand(rowKey, 'model')}
                          className="p-0.5 hover:bg-muted rounded"
                        >
                          {isExpanded === 'model' ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                        </button>
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
                  />
                )}
                {isExpanded === 'model' && hasModelBreakdown && (
                  <ModelBreakdownRows breakdown={row.modelBreakdown!} />
                )}
              </>
            );
          })
        )}
      </TableBody>
    </Table>
  );
}

function SessionTable({ data }: { data: SessionRow[] }) {
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
          <TableHead>Last Active</TableHead>
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
            const rowKey = row.sessionId;
            const isExpanded = expanded[rowKey];
            const hasModelBreakdown = row.modelBreakdown && row.modelBreakdown.length > 0;

            return (
              <>
                <TableRow
                  key={rowKey}
                  className={isExpanded ? 'bg-muted/50' : ''}
                >
                  <TableCell className="font-mono text-xs">
                    <div className="flex items-center gap-1">
                      {hasModelBreakdown && (
                        <button
                          onClick={() => toggleExpand(rowKey)}
                          className="p-0.5 hover:bg-muted rounded"
                        >
                          {isExpanded ? (
                            <ChevronDown className="h-4 w-4" />
                          ) : (
                            <ChevronRight className="h-4 w-4" />
                          )}
                        </button>
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
                  <TableCell className="text-muted-foreground">
                    {row.lastActivity
                      ? new Date(row.lastActivity).toLocaleDateString()
                      : '-'}
                  </TableCell>
                </TableRow>
                {isExpanded && hasModelBreakdown && (
                  <ModelBreakdownRows breakdown={row.modelBreakdown!} />
                )}
              </>
            );
          })
        )}
      </TableBody>
    </Table>
  );
}

function BlockTable({ data }: { data: BlockRow[] }) {
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
          <TableHead>Models</TableHead>
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
          data.map((row) => (
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
              <TableCell className="text-muted-foreground">
                {row.modelsUsed?.join(', ') ?? '-'}
              </TableCell>
            </TableRow>
          ))
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

export function UsageTable({ data, type, snapshot, source }: UsageTableProps) {
  switch (type) {
    case 'daily':
      return <DailyTable data={data as DailyRow[]} snapshot={snapshot} source={source} />;
    case 'monthly':
      return <MonthlyTable data={data as MonthlyRow[]} snapshot={snapshot} source={source} />;
    case 'session':
      return <SessionTable data={data as SessionRow[]} />;
    case 'blocks':
      return <BlockTable data={data as BlockRow[]} />;
  }
}
