import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  useReactTable,
} from '@tanstack/react-table';
import type { DailyRow, MonthlyRow, SessionRow, BlockRow } from '../api/types';

interface UsageTableProps {
  data: DailyRow[] | MonthlyRow[] | SessionRow[] | BlockRow[];
  type: 'daily' | 'monthly' | 'session' | 'blocks';
}

const dailyColumns = createColumnHelper<DailyRow>();
const monthlyColumns = createColumnHelper<MonthlyRow>();
const sessionColumns = createColumnHelper<SessionRow>();
const blockColumns = createColumnHelper<BlockRow>();

const dailyColDefs = [
  dailyColumns.accessor('date', { header: 'Date' }),
  dailyColumns.accessor('costFormatted', { header: 'Cost' }),
  dailyColumns.accessor('totalTokens', {
    header: 'Tokens',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
  dailyColumns.accessor('inputTokens', {
    header: 'Input',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
  dailyColumns.accessor('outputTokens', {
    header: 'Output',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
];

const monthlyColDefs = [
  monthlyColumns.accessor('month', { header: 'Month' }),
  monthlyColumns.accessor('costFormatted', { header: 'Cost' }),
  monthlyColumns.accessor('totalTokens', {
    header: 'Tokens',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
  monthlyColumns.accessor('inputTokens', {
    header: 'Input',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
  monthlyColumns.accessor('outputTokens', {
    header: 'Output',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
];

const sessionColDefs = [
  sessionColumns.accessor('sessionId', {
    header: 'Session',
    cell: info => (info.getValue() ?? '').slice(0, 8),
  }),
  sessionColumns.accessor('projectPath', {
    header: 'Project',
    cell: info => info.getValue()?.split('/').pop() ?? '-',
  }),
  sessionColumns.accessor('costFormatted', { header: 'Cost' }),
  sessionColumns.accessor('totalTokens', {
    header: 'Tokens',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
  sessionColumns.accessor('lastActivity', {
    header: 'Last Active',
    cell: info => {
      const val = info.getValue();
      return val ? new Date(val).toLocaleDateString() : '-';
    },
  }),
];

const blockColDefs = [
  blockColumns.accessor('blockId', {
    header: 'Block',
    cell: info => (info.getValue() ?? '').slice(0, 8),
  }),
  blockColumns.accessor('startTime', {
    header: 'Start',
    cell: info => {
      const val = info.getValue();
      return val ? new Date(val).toLocaleString() : '-';
    },
  }),
  blockColumns.accessor('costFormatted', { header: 'Cost' }),
  blockColumns.accessor('totalTokens', {
    header: 'Tokens',
    cell: info => (info.getValue() ?? 0).toLocaleString(),
  }),
  blockColumns.accessor('isActive', {
    header: 'Active',
    cell: info => (info.getValue() ? '✓' : ''),
  }),
];

export function UsageTable({ data, type }: UsageTableProps) {
  const columns = type === 'daily' ? dailyColDefs
    : type === 'monthly' ? monthlyColDefs
    : type === 'session' ? sessionColDefs
    : blockColDefs;

  const table = useReactTable({
    data: data as any[],
    columns: columns as any,
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <div className="rounded-md border">
      <table className="w-full caption-bottom text-sm">
        <thead className="[&_tr]:border-b">
          {table.getHeaderGroups().map(headerGroup => (
            <tr key={headerGroup.id} className="border-b transition-colors hover:bg-muted/50">
              {headerGroup.headers.map(header => (
                <th
                  key={header.id}
                  className="h-12 px-4 text-left align-middle font-medium text-muted-foreground"
                >
                  {header.isPlaceholder
                    ? null
                    : flexRender(header.column.columnDef.header, header.getContext())}
                </th>
              ))}
            </tr>
          ))}
        </thead>
        <tbody className="[&_tr:last-child]:border-0">
          {table.getRowModel().rows?.length ? (
            table.getRowModel().rows.map(row => (
              <tr
                key={row.id}
                className="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted"
              >
                {row.getVisibleCells().map(cell => (
                  <td key={cell.id} className="p-4 align-middle">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))
          ) : (
            <tr>
              <td colSpan={columns.length} className="h-24 text-center text-muted-foreground">
                No data available
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
