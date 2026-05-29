import type { DailyRow, MonthlyRow, SessionRow, BlockRow } from '../api/types';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

interface UsageTableProps {
  data: DailyRow[] | MonthlyRow[] | SessionRow[] | BlockRow[];
  type: 'daily' | 'monthly' | 'session' | 'blocks';
}

function fmt(n: number | undefined): string {
  return (n ?? 0).toLocaleString();
}

function DailyTable({ data }: { data: DailyRow[] }) {
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
          data.map((row) => (
            <TableRow key={row.date}>
              <TableCell className="font-medium">{row.date}</TableCell>
              <TableCell className="text-right">{fmt(row.totalTokens)}</TableCell>
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

function MonthlyTable({ data }: { data: MonthlyRow[] }) {
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
          data.map((row) => (
            <TableRow key={row.month}>
              <TableCell className="font-medium">{row.month}</TableCell>
              <TableCell className="text-right">{fmt(row.totalTokens)}</TableCell>
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

function SessionTable({ data }: { data: SessionRow[] }) {
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
          data.map((row) => (
            <TableRow key={row.sessionId}>
              <TableCell className="font-mono text-xs">
                {row.sessionId.slice(0, 8)}
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
          ))
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
          <TableHead>Block</TableHead>
          <TableHead>Start</TableHead>
          <TableHead className="text-right">Total</TableHead>
          <TableHead>Active</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {data.length === 0 ? (
          <TableRow>
            <TableCell colSpan={4} className="h-24 text-center text-muted-foreground">
              No data available
            </TableCell>
          </TableRow>
        ) : (
          data.map((row) => (
            <TableRow key={row.blockId}>
              <TableCell className="font-mono text-xs">
                {row.blockId.slice(0, 8)}
              </TableCell>
              <TableCell className="text-muted-foreground">
                {new Date(row.startTime).toLocaleString()}
              </TableCell>
              <TableCell className="text-right">{fmt(row.totalTokens)}</TableCell>
              <TableCell>{row.isActive ? 'Yes' : ''}</TableCell>
            </TableRow>
          ))
        )}
      </TableBody>
    </Table>
  );
}

export function UsageTable({ data, type }: UsageTableProps) {
  switch (type) {
    case 'daily':
      return <DailyTable data={data as DailyRow[]} />;
    case 'monthly':
      return <MonthlyTable data={data as MonthlyRow[]} />;
    case 'session':
      return <SessionTable data={data as SessionRow[]} />;
    case 'blocks':
      return <BlockTable data={data as BlockRow[]} />;
  }
}
