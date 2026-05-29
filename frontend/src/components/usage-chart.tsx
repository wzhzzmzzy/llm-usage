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
import type { DailyRow, MonthlyRow, SessionRow, BlockRow } from '../api/types';

interface UsageChartProps {
  data: DailyRow[] | MonthlyRow[] | SessionRow[] | BlockRow[];
  type: 'daily' | 'monthly' | 'session' | 'blocks';
}

function CustomTooltip({ active, payload, label }: any) {
  if (!active || !payload) return null;
  return (
    <div className="rounded-lg border bg-background p-3 shadow-sm">
      <p className="mb-2 font-medium">{label}</p>
      {payload.map((entry: any) => (
        <p key={entry.name} className="text-sm" style={{ color: entry.color }}>
          {entry.name}: {entry.value.toLocaleString()}
        </p>
      ))}
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

export function UsageChart({ data, type }: UsageChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-muted-foreground">
        No data available
      </div>
    );
  }

  let chartData: { name: string; input: number; cache: number; output: number }[] = [];

  switch (type) {
    case 'daily':
      chartData = (data as DailyRow[]).slice().reverse().map((row) => ({
        name: row.date,
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
      }));
      break;
    case 'monthly':
      chartData = (data as MonthlyRow[]).slice().reverse().map((row) => ({
        name: row.month,
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
      }));
      break;
    case 'session':
      chartData = (data as SessionRow[]).slice().reverse().map((row) => ({
        name: row.sessionId.slice(0, 8),
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
      }));
      break;
    case 'blocks':
      chartData = (data as BlockRow[]).slice().reverse().map((row) => ({
        name: formatBlockLabel(row.startTime),
        input: row.inputTokens ?? 0,
        cache: row.cacheReadTokens ?? 0,
        output: row.outputTokens ?? 0,
      }));
      break;
  }

  return (
    <ResponsiveContainer width="100%" height={350}>
      <BarChart data={chartData}>
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
      </BarChart>
    </ResponsiveContainer>
  );
}
