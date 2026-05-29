import { describe, it, expect } from 'vitest';
import type { DailyRow, MonthlyRow, SessionRow, ModelBreakdown } from '../api/types';

const mockDailyRows: DailyRow[] = [
  {
    date: '2026-05-29',
    totalTokens: 150000,
    inputTokens: 100000,
    cacheReadTokens: 20000,
    outputTokens: 30000,
    requestCount: 10,
    modelsUsed: ['gpt-5.5', 'claude-sonnet-4'],
    modelBreakdown: [
      {
        model: 'gpt-5.5',
        inputTokens: 60000,
        cacheReadTokens: 15000,
        outputTokens: 20000,
        totalTokens: 95000,
        requestCount: 6,
      },
      {
        model: 'claude-sonnet-4',
        inputTokens: 40000,
        cacheReadTokens: 5000,
        outputTokens: 10000,
        totalTokens: 55000,
        requestCount: 4,
      },
    ],
  },
  {
    date: '2026-05-28',
    totalTokens: 80000,
    inputTokens: 50000,
    cacheReadTokens: 10000,
    outputTokens: 20000,
    requestCount: 5,
    modelsUsed: ['gpt-5.5'],
  },
];

const mockMonthlyRows: MonthlyRow[] = [
  {
    month: '2026-05',
    totalTokens: 500000,
    inputTokens: 300000,
    cacheReadTokens: 50000,
    outputTokens: 150000,
    requestCount: 50,
    modelsUsed: ['gpt-5.5', 'claude-sonnet-4'],
    modelBreakdown: [
      {
        model: 'gpt-5.5',
        inputTokens: 200000,
        cacheReadTokens: 30000,
        outputTokens: 100000,
        totalTokens: 330000,
        requestCount: 30,
      },
      {
        model: 'claude-sonnet-4',
        inputTokens: 100000,
        cacheReadTokens: 20000,
        outputTokens: 50000,
        totalTokens: 170000,
        requestCount: 20,
      },
    ],
  },
];

const mockSessionRows: SessionRow[] = [
  {
    sessionId: 'ses_abc123def456',
    projectPath: '/Users/test/my-project',
    totalTokens: 35000,
    inputTokens: 25000,
    cacheReadTokens: 5000,
    outputTokens: 5000,
    requestCount: 3,
    lastActivity: '2026-05-29T10:00:00Z',
    modelsUsed: ['gpt-5.5'],
    modelBreakdown: [
      {
        model: 'gpt-5.5',
        inputTokens: 25000,
        cacheReadTokens: 5000,
        outputTokens: 5000,
        totalTokens: 35000,
        requestCount: 3,
      },
    ],
  },
];

describe('DailyRow types', () => {
  it('should have correct structure', () => {
    const row = mockDailyRows[0];
    expect(row.date).toBe('2026-05-29');
    expect(row.totalTokens).toBe(150000);
    expect(row.inputTokens).toBe(100000);
    expect(row.cacheReadTokens).toBe(20000);
    expect(row.outputTokens).toBe(30000);
    expect(row.requestCount).toBe(10);
    expect(row.modelsUsed).toEqual(['gpt-5.5', 'claude-sonnet-4']);
  });

  it('should have model breakdown', () => {
    const row = mockDailyRows[0];
    expect(row.modelBreakdown).toHaveLength(2);
    expect(row.modelBreakdown![0].model).toBe('gpt-5.5');
    expect(row.modelBreakdown![0].inputTokens).toBe(60000);
    expect(row.modelBreakdown![0].cacheReadTokens).toBe(15000);
    expect(row.modelBreakdown![0].outputTokens).toBe(20000);
    expect(row.modelBreakdown![0].totalTokens).toBe(95000);
    expect(row.modelBreakdown![0].requestCount).toBe(6);
  });

  it('should handle optional model breakdown', () => {
    const row = mockDailyRows[1];
    expect(row.modelBreakdown).toBeUndefined();
  });

  it('should have consistent totals', () => {
    const row = mockDailyRows[0];
    const breakdownTotal = row.modelBreakdown!.reduce((sum, m) => sum + m.totalTokens, 0);
    expect(breakdownTotal).toBe(row.totalTokens);
  });
});

describe('MonthlyRow types', () => {
  it('should have correct structure', () => {
    const row = mockMonthlyRows[0];
    expect(row.month).toBe('2026-05');
    expect(row.totalTokens).toBe(500000);
    expect(row.inputTokens).toBe(300000);
    expect(row.cacheReadTokens).toBe(50000);
    expect(row.outputTokens).toBe(150000);
    expect(row.requestCount).toBe(50);
  });

  it('should have model breakdown', () => {
    const row = mockMonthlyRows[0];
    expect(row.modelBreakdown).toHaveLength(2);
  });
});

describe('SessionRow types', () => {
  it('should have correct structure', () => {
    const row = mockSessionRows[0];
    expect(row.sessionId).toBe('ses_abc123def456');
    expect(row.projectPath).toBe('/Users/test/my-project');
    expect(row.totalTokens).toBe(35000);
    expect(row.inputTokens).toBe(25000);
    expect(row.cacheReadTokens).toBe(5000);
    expect(row.outputTokens).toBe(5000);
    expect(row.requestCount).toBe(3);
    expect(row.lastActivity).toBe('2026-05-29T10:00:00Z');
  });

  it('should have model breakdown', () => {
    const row = mockSessionRows[0];
    expect(row.modelBreakdown).toHaveLength(1);
    expect(row.modelBreakdown![0].model).toBe('gpt-5.5');
  });
});

describe('ModelBreakdown types', () => {
  it('should have correct structure', () => {
    const breakdown: ModelBreakdown = {
      model: 'gpt-5.5',
      inputTokens: 1000,
      cacheReadTokens: 200,
      outputTokens: 300,
      totalTokens: 1500,
      requestCount: 5,
    };
    expect(breakdown.model).toBe('gpt-5.5');
    expect(breakdown.inputTokens).toBe(1000);
    expect(breakdown.cacheReadTokens).toBe(200);
    expect(breakdown.outputTokens).toBe(300);
    expect(breakdown.totalTokens).toBe(1500);
    expect(breakdown.requestCount).toBe(5);
  });

  it('should have consistent totals', () => {
    const breakdown: ModelBreakdown = {
      model: 'gpt-5.5',
      inputTokens: 1000,
      cacheReadTokens: 200,
      outputTokens: 300,
      totalTokens: 1500,
      requestCount: 5,
    };
    // totalTokens should equal inputTokens + cacheReadTokens + outputTokens
    expect(breakdown.totalTokens).toBe(
      breakdown.inputTokens + breakdown.cacheReadTokens + breakdown.outputTokens
    );
  });
});

describe('table data safety', () => {
  it('should handle undefined values in daily rows', () => {
    const unsafeRow: Partial<DailyRow> = {
      date: '2026-05-28',
      totalTokens: undefined,
      inputTokens: undefined,
      cacheReadTokens: undefined,
      outputTokens: undefined,
    };
    const safeTotal = (unsafeRow.totalTokens ?? 0).toLocaleString();
    const safeInput = (unsafeRow.inputTokens ?? 0).toLocaleString();
    const safeCache = (unsafeRow.cacheReadTokens ?? 0).toLocaleString();
    const safeOutput = (unsafeRow.outputTokens ?? 0).toLocaleString();
    expect(safeTotal).toBe('0');
    expect(safeInput).toBe('0');
    expect(safeCache).toBe('0');
    expect(safeOutput).toBe('0');
  });

  it('should handle null values in session rows', () => {
    const unsafeRow: { sessionId: string | null; projectPath: string | null; lastActivity: string | null } = {
      sessionId: null,
      projectPath: null,
      lastActivity: null,
    };
    const safeId = (unsafeRow.sessionId ?? '').slice(0, 8);
    const safePath = unsafeRow.projectPath?.split('/').pop() ?? '-';
    const safeDate = unsafeRow.lastActivity ? new Date(unsafeRow.lastActivity).toLocaleDateString() : '-';
    expect(safeId).toBe('');
    expect(safePath).toBe('-');
    expect(safeDate).toBe('-');
  });

  it('should handle empty modelsUsed', () => {
    const models: string[] | undefined = undefined;
    const safeModels = models && models.length > 0 ? models.join(', ') : '-';
    expect(safeModels).toBe('-');
  });
});
