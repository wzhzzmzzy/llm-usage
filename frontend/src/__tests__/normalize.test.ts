import { describe, it, expect } from 'vitest';

const mockDailyResponse = {
  daily: [
    {
      period: '2026-05-27',
      inputTokens: 50000,
      outputTokens: 10000,
      cacheCreationTokens: 0,
      cacheReadTokens: 5000,
      totalCost: 0.15,
      totalTokens: 60000,
      modelsUsed: ['claude-sonnet-4-5'],
      modelBreakdowns: [
        {
          modelName: 'claude-sonnet-4-5',
          inputTokens: 50000,
          outputTokens: 10000,
          cacheCreationTokens: 0,
          cacheReadTokens: 5000,
          cost: 0.15,
        },
      ],
    },
    {
      period: '2026-05-28',
      inputTokens: 100000,
      outputTokens: 20000,
      cacheCreationTokens: 1000,
      cacheReadTokens: 10000,
      totalCost: 0.35,
      totalTokens: 120000,
      modelsUsed: ['claude-sonnet-4-5', 'gemini-3-pro-preview'],
    },
  ],
};

const mockSessionResponse = {
  session: [
    {
      period: 'ses_abc123def456',
      inputTokens: 30000,
      outputTokens: 5000,
      cacheCreationTokens: 0,
      cacheReadTokens: 2000,
      totalCost: 0.08,
      totalTokens: 35000,
      modelsUsed: ['claude-sonnet-4-5'],
      lastActivity: '2026-05-28',
      projectPath: '/Users/test/my-project',
      metadata: {
        lastActivity: '2026-05-28',
        projectPath: '/Users/test/my-project',
      },
    },
  ],
};

const mockBlocksResponse = {
  blocks: [
    {
      id: '2026-05-28T09:00:00.000Z',
      startTime: '2026-05-28T09:00:00.000Z',
      endTime: '2026-05-28T12:00:00.000Z',
      actualEndTime: '2026-05-28T11:30:00.000Z',
      isActive: false,
      isGap: false,
      models: ['claude-sonnet-4-5'],
      costUsd: 5.25,
      totalTokens: 500000,
      tokenCounts: {
        inputTokens: 400000,
        outputTokens: 100000,
        cacheCreationInputTokens: 0,
        cacheReadInputTokens: 50000,
      },
      entries: 15,
    },
    {
      id: '2026-05-28T12:00:00.000Z',
      startTime: '2026-05-28T12:00:00.000Z',
      endTime: null,
      actualEndTime: null,
      isActive: true,
      isGap: false,
      models: ['claude-sonnet-4-5'],
      costUsd: 2.10,
      totalTokens: 200000,
      tokenCounts: {
        inputTokens: 150000,
        outputTokens: 50000,
        cacheCreationInputTokens: 0,
        cacheReadInputTokens: 20000,
      },
      entries: 5,
    },
    {
      id: 'gap-2026-05-28T08:00:00.000Z',
      startTime: '2026-05-28T08:00:00.000Z',
      endTime: '2026-05-28T09:00:00.000Z',
      actualEndTime: null,
      isActive: false,
      isGap: true,
      models: [],
      costUsd: 0,
      totalTokens: 0,
      tokenCounts: null,
      entries: 0,
    },
  ],
};

function normalizeDaily(data: typeof mockDailyResponse) {
  return data.daily.map(row => ({
    date: row.period,
    costUsdNumber: row.totalCost,
    costFormatted: `$${row.totalCost.toFixed(2)}`,
    totalTokens: row.totalTokens ?? (row.inputTokens + row.outputTokens),
    inputTokens: row.inputTokens,
    outputTokens: row.outputTokens,
    cacheCreationTokens: row.cacheCreationTokens,
    cacheReadTokens: row.cacheReadTokens,
    modelsUsed: row.modelsUsed,
  }));
}

function normalizeSession(data: typeof mockSessionResponse) {
  return data.session.map(row => ({
    sessionId: row.period,
    projectPath: row.projectPath ?? row.metadata?.projectPath ?? null,
    costUsdNumber: row.totalCost,
    costFormatted: `$${row.totalCost.toFixed(2)}`,
    totalTokens: row.totalTokens ?? (row.inputTokens + row.outputTokens),
    inputTokens: row.inputTokens,
    outputTokens: row.outputTokens,
    lastActivity: row.lastActivity ?? row.metadata?.lastActivity ?? null,
    modelsUsed: row.modelsUsed,
  }));
}

function normalizeBlocks(data: typeof mockBlocksResponse) {
  return data.blocks
    .filter(row => !row.isGap)
    .map(row => ({
      blockId: row.id,
      startTime: row.startTime,
      endTime: row.actualEndTime ?? row.endTime,
      costUsdNumber: row.costUsd ?? 0,
      costFormatted: `$${(row.costUsd ?? 0).toFixed(2)}`,
      totalTokens: row.totalTokens ?? 0,
      inputTokens: row.tokenCounts?.inputTokens ?? 0,
      outputTokens: row.tokenCounts?.outputTokens ?? 0,
      isActive: row.isActive,
      modelsUsed: row.models ?? [],
    }));
}

describe('normalizeDaily', () => {
  it('should normalize daily rows', () => {
    const result = normalizeDaily(mockDailyResponse);
    expect(result).toHaveLength(2);
  });

  it('should parse date correctly', () => {
    const result = normalizeDaily(mockDailyResponse);
    expect(result[0].date).toBe('2026-05-27');
  });

  it('should calculate totalTokens', () => {
    const result = normalizeDaily(mockDailyResponse);
    expect(result[0].totalTokens).toBe(60000);
  });

  it('should format cost', () => {
    const result = normalizeDaily(mockDailyResponse);
    expect(result[0].costFormatted).toBe('$0.15');
  });

  it('should handle modelsUsed', () => {
    const result = normalizeDaily(mockDailyResponse);
    expect(result[0].modelsUsed).toEqual(['claude-sonnet-4-5']);
    expect(result[1].modelsUsed).toEqual(['claude-sonnet-4-5', 'gemini-3-pro-preview']);
  });
});

describe('normalizeSession', () => {
  it('should normalize session rows', () => {
    const result = normalizeSession(mockSessionResponse);
    expect(result).toHaveLength(1);
  });

  it('should parse sessionId', () => {
    const result = normalizeSession(mockSessionResponse);
    expect(result[0].sessionId).toBe('ses_abc123def456');
  });

  it('should extract projectPath from metadata', () => {
    const result = normalizeSession(mockSessionResponse);
    expect(result[0].projectPath).toBe('/Users/test/my-project');
  });

  it('should extract lastActivity from metadata', () => {
    const result = normalizeSession(mockSessionResponse);
    expect(result[0].lastActivity).toBe('2026-05-28');
  });
});

describe('normalizeBlocks', () => {
  it('should filter out gap blocks', () => {
    const result = normalizeBlocks(mockBlocksResponse);
    expect(result).toHaveLength(2);
  });

  it('should parse blockId', () => {
    const result = normalizeBlocks(mockBlocksResponse);
    expect(result[0].blockId).toBe('2026-05-28T09:00:00.000Z');
  });

  it('should extract tokenCounts', () => {
    const result = normalizeBlocks(mockBlocksResponse);
    expect(result[0].inputTokens).toBe(400000);
    expect(result[0].outputTokens).toBe(100000);
  });

  it('should handle isActive', () => {
    const result = normalizeBlocks(mockBlocksResponse);
    expect(result[0].isActive).toBe(false);
    expect(result[1].isActive).toBe(true);
  });

  it('should use actualEndTime for endTime', () => {
    const result = normalizeBlocks(mockBlocksResponse);
    expect(result[0].endTime).toBe('2026-05-28T11:30:00.000Z');
  });
});

describe('table data safety', () => {
  it('should handle undefined values in daily rows', () => {
    const unsafeRow = {
      date: '2026-05-28',
      costFormatted: undefined,
      totalTokens: undefined,
      inputTokens: undefined,
      outputTokens: undefined,
    };
    const safeCost = unsafeRow.costFormatted ?? '$0.00';
    const safeTokens = (unsafeRow.totalTokens ?? 0).toLocaleString();
    expect(safeCost).toBe('$0.00');
    expect(safeTokens).toBe('0');
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
});
