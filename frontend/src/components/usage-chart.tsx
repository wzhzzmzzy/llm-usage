import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';
import type { DailyRow, MonthlyRow, SessionRow, BlockRow, Snapshot, PricingMap, ModelBreakdown } from '../api/types';

interface UsageChartProps {
  data: DailyRow[] | MonthlyRow[] | SessionRow[] | BlockRow[];
  type: 'daily' | 'monthly' | 'session' | 'blocks';
  snapshot?: Snapshot;
  segmentMode?: 'token-type' | 'agent-source';
  pricing?: PricingMap | null;
}

type AgentSourceKey = 'claude' | 'codex' | 'gemini' | 'opencode';

const AGENT_SOURCES: { key: AgentSourceKey; label: string; color: string }[] = [
  { key: 'claude', label: 'Claude', color: 'var(--color-chart-1)' },
  { key: 'codex', label: 'Codex', color: 'var(--color-chart-2)' },
  { key: 'gemini', label: 'Gemini', color: 'var(--color-chart-3)' },
  { key: 'opencode', label: 'OpenCode', color: 'var(--color-chart-4)' },
];

const DEFAULT_PRICING = { input: 3e-6, output: 15e-6, cacheCreate: 3.75e-6, cacheRead: 0.3e-6 };

function findModelPricing(model: string, pricing: PricingMap | null | undefined) {
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

function fmtCost(n: number): string {
  if (n === 0) return '$0';
  if (n < 0.01) return `$${n.toFixed(4)}`;
  if (n < 1) return `$${n.toFixed(3)}`;
  return `$${n.toFixed(2)}`;
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
  return inputTokens * DEFAULT_PRICING.input + outputTokens * DEFAULT_PRICING.output + cacheReadTokens * DEFAULT_PRICING.cacheRead;
}

function CustomTooltip({ active, payload, label }: any) {
  if (!active || !payload) return null;
  const costEntry = payload.find((e: any) => e.dataKey === 'cost');
  return (
    <div className="rounded-lg border bg-background p-3 shadow-sm">
      <p className="mb-2 font-medium">{label}</p>
      {payload.filter((e: any) => e.dataKey !== 'cost').map((entry: any) => (
        <p key={entry.name} className="text-sm" style={{ color: entry.color }}>
          {entry.name}: {entry.value.toLocaleString()}
        </p>
      ))}
      {costEntry && (
        <p className="text-sm font-medium mt-1 pt-1 border-t">
          Est. Cost: {fmtCost(costEntry.value)}
        </p>
      )}
    </div>
  );
}

function formatBlockLabel(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function getChartDataByTokenType(
  data: DailyRow[] | MonthlyRow[] | SessionRow[] | BlockRow[],
  type: string,
  pricing?: PricingMap | null,
): { name: string; input: number; cache: number; output: number; cost: number }[] {
  switch (type) {
    case 'daily':
      return (data as DailyRow[]).slice().reverse().map((row) => ({
        name: row.date,
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
        cost: estimateRowCost(row.inputTokens ?? 0, row.outputTokens ?? 0, row.cacheReadTokens ?? 0, row.modelBreakdown, pricing),
      }));
    case 'monthly':
      return (data as MonthlyRow[]).slice().reverse().map((row) => ({
        name: row.month,
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
        cost: estimateRowCost(row.inputTokens ?? 0, row.outputTokens ?? 0, row.cacheReadTokens ?? 0, row.modelBreakdown, pricing),
      }));
    case 'session':
      return (data as SessionRow[]).slice().reverse().map((row) => ({
        name: row.sessionId.slice(0, 8),
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
        cost: estimateRowCost(row.inputTokens ?? 0, row.outputTokens ?? 0, row.cacheReadTokens ?? 0, row.modelBreakdown, pricing),
      }));
    case 'blocks':
      return (data as BlockRow[]).slice().reverse().map((row) => ({
        name: formatBlockLabel(row.startTime),
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
        cost: estimateRowCost(row.inputTokens ?? 0, row.outputTokens ?? 0, row.cacheReadTokens ?? 0, null, pricing),
      }));
    default:
      return [];
  }
}

function getChartDataByAgentSource(
  snapshot: Snapshot,
  type: string
): { name: string; claude: number; codex: number; gemini: number; opencode: number }[] {
  const sources: AgentSourceKey[] = ['claude', 'codex', 'gemini', 'opencode'];

  switch (type) {
    case 'daily': {
      const dateMap = new Map<string, { claude: number; codex: number; gemini: number; opencode: number }>();

      for (const src of sources) {
        const report = snapshot.daily?.[`${src}_daily`];
        if (!report) continue;

        for (const row of report.days) {
          const existing = dateMap.get(row.date) ?? { claude: 0, codex: 0, gemini: 0, opencode: 0 };
          existing[src] = (row.inputTokens ?? 0) + (row.cacheReadTokens ?? 0) + (row.outputTokens ?? 0);
          dateMap.set(row.date, existing);
        }
      }

      return Array.from(dateMap.entries())
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([date, values]) => ({ name: date, ...values }));
    }
    case 'monthly': {
      const monthMap = new Map<string, { claude: number; codex: number; gemini: number; opencode: number }>();

      for (const src of sources) {
        const report = snapshot.monthly?.[`${src}_monthly`];
        if (!report) continue;

        for (const row of report.months) {
          const existing = monthMap.get(row.month) ?? { claude: 0, codex: 0, gemini: 0, opencode: 0 };
          existing[src] = (row.inputTokens ?? 0) + (row.cacheReadTokens ?? 0) + (row.outputTokens ?? 0);
          monthMap.set(row.month, existing);
        }
      }

      return Array.from(monthMap.entries())
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([month, values]) => ({ name: month, ...values }));
    }
    case 'session': {
      const sessionMap = new Map<string, { claude: number; codex: number; gemini: number; opencode: number }>();

      for (const src of sources) {
        const report = snapshot.session?.[`${src}_session`];
        if (!report) continue;

        for (const row of report.sessions) {
          const key = row.sessionId.slice(0, 8);
          const existing = sessionMap.get(key) ?? { claude: 0, codex: 0, gemini: 0, opencode: 0 };
          existing[src] = (row.inputTokens ?? 0) + (row.cacheReadTokens ?? 0) + (row.outputTokens ?? 0);
          sessionMap.set(key, existing);
        }
      }

      return Array.from(sessionMap.entries())
        .map(([name, values]) => ({ name, ...values }));
    }
    case 'blocks': {
      const blockMap = new Map<string, { claude: number; codex: number; gemini: number; opencode: number }>();

      for (const src of sources) {
        const report = snapshot.blocks?.[`${src}_blocks`];
        if (!report) continue;

        for (const row of report.blocks) {
          const key = row.startTime;
          const existing = blockMap.get(key) ?? { claude: 0, codex: 0, gemini: 0, opencode: 0 };
          existing[src] = (row.inputTokens ?? 0) + (row.cacheReadTokens ?? 0) + (row.outputTokens ?? 0);
          blockMap.set(key, existing);
        }
      }

      return Array.from(blockMap.entries())
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([time, values]) => ({ name: formatBlockLabel(time), ...values }));
    }
    default:
      return [];
  }
}

export function UsageChart({ data, type, snapshot, segmentMode = 'token-type', pricing }: UsageChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-muted-foreground">
        No data available
      </div>
    );
  }

  const isAgentSourceMode = segmentMode === 'agent-source' && snapshot;

  const chartData = isAgentSourceMode
    ? getChartDataByAgentSource(snapshot, type)
    : getChartDataByTokenType(data, type, pricing);

  if (chartData.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-muted-foreground">
        No data available
      </div>
    );
  }

  if (isAgentSourceMode) {
    const agentData = chartData as { name: string; claude: number; codex: number; gemini: number; opencode: number }[];
    return (
      <ResponsiveContainer width="100%" height={350}>
        <BarChart data={agentData}>
          <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
          <XAxis
            dataKey="name"
            tick={{ fontSize: 12 }}
            className="text-muted-foreground"
          />
          <YAxis
            tick={{ fontSize: 12 }}
            className="text-muted-foreground"
            tickFormatter={(v) => {
              if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
              if (v >= 1_000) return `${(v / 1_000).toFixed(0)}K`;
              return v.toString();
            }}
          />
          <Tooltip content={<CustomTooltip />} />
          <Legend />
          {AGENT_SOURCES.map((src, index) => (
            <Bar
              key={src.key}
              dataKey={src.key}
              name={src.label}
              stackId="tokens"
              fill={src.color}
              radius={index === AGENT_SOURCES.length - 1 ? [4, 4, 0, 0] : [0, 0, 0, 0]}
            />
          ))}
        </BarChart>
      </ResponsiveContainer>
    );
  }

  const tokenData = chartData as { name: string; input: number; cache: number; output: number; cost: number }[];
  return (
    <ResponsiveContainer width="100%" height={350}>
      <BarChart data={tokenData}>
        <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
        <XAxis
          dataKey="name"
          tick={{ fontSize: 12 }}
          className="text-muted-foreground"
        />
        <YAxis
          tick={{ fontSize: 12 }}
          className="text-muted-foreground"
          tickFormatter={(v) => {
            if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M`;
            if (v >= 1_000) return `${(v / 1_000).toFixed(0)}K`;
            return v.toString();
          }}
        />
        <Tooltip content={<CustomTooltip />} />
        <Legend />
        <Bar
          dataKey="input"
          name="Input"
          stackId="tokens"
          fill="var(--color-chart-1)"
          radius={[0, 0, 0, 0]}
        />
        <Bar
          dataKey="cache"
          name="Cache Hit"
          stackId="tokens"
          fill="var(--color-chart-2)"
          radius={[0, 0, 0, 0]}
        />
        <Bar
          dataKey="output"
          name="Output"
          stackId="tokens"
          fill="var(--color-chart-3)"
          radius={[4, 4, 0, 0]}
        />
        <Bar dataKey="cost" name="Est. Cost" hide />
      </BarChart>
    </ResponsiveContainer>
  );
}
