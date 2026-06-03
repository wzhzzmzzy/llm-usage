import { useMemo, useState, useCallback, useRef, useEffect } from 'react';
import type { DailyReport, PricingMap, ModelBreakdown } from '../api/types';

interface ContributionCalendarProps {
  data: Record<string, DailyReport>;
  onDayClick?: (date: string) => void;
  pricing?: PricingMap | null;
}

interface DayData {
  date: string;
  tokens: number;
  inputTokens: number;
  cacheReadTokens: number;
  outputTokens: number;
  models: string[];
  modelBreakdown?: ModelBreakdown[];
}

interface TooltipState {
  day: DayData;
  x: number;
  y: number;
}

const WEEKS_TO_SHOW = 53;
const CELL_SIZE = 14;
const CELL_GAP = 3;

const COLORS = [
  '#ebedf0',
  '#9be9a8',
  '#40c463',
  '#30a14e',
  '#216e39',
];

const MONTH_LABELS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

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

function estimateDayCost(day: DayData, pricing: PricingMap | null | undefined): number {
  if (day.modelBreakdown && day.modelBreakdown.length > 0) {
    return day.modelBreakdown.reduce((total, mb) => {
      const p = findModelPricing(mb.model, pricing);
      return total + mb.inputTokens * p.input + mb.outputTokens * p.output + mb.cacheReadTokens * p.cacheRead;
    }, 0);
  }
  const p = DEFAULT_PRICING;
  return day.inputTokens * p.input + day.outputTokens * p.output + day.cacheReadTokens * p.cacheRead;
}

function getIntensity(tokens: number): number {
  if (tokens === 0) return 0;
  if (tokens < 1_000_000) return 1;
  if (tokens < 10_000_000) return 2;
  if (tokens < 100_000_000) return 3;
  return 4;
}

export function ContributionCalendar({ data, onDayClick, pricing }: ContributionCalendarProps) {
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [tooltipStyle, setTooltipStyle] = useState<React.CSSProperties>({});

  const { grid } = useMemo(() => {
    const today = new Date();
    const start = new Date(today);
    start.setDate(start.getDate() - (WEEKS_TO_SHOW * 7 - 1) - start.getDay());

    const grid: DayData[][] = [];
    let current = new Date(start);
    let week: DayData[] = [];

    while (current <= today) {
      const dateStr = current.toISOString().split('T')[0];
      const report = Object.values(data).find(r =>
        r?.days?.some(d => d.date === dateStr)
      );
      const day = report?.days?.find(d => d.date === dateStr);

      week.push({
        date: dateStr,
        tokens: day?.totalTokens ?? 0,
        inputTokens: day?.inputTokens ?? 0,
        cacheReadTokens: day?.cacheReadTokens ?? 0,
        outputTokens: day?.outputTokens ?? 0,
        models: day?.modelsUsed ?? [],
        modelBreakdown: day?.modelBreakdown,
      });

      if (week.length === 7) {
        grid.push(week);
        week = [];
      }

      current.setDate(current.getDate() + 1);
    }

    if (week.length > 0) {
      grid.push(week);
    }

    return { grid };
  }, [data]);

  const months = useMemo(() => {
    const result: { label: string; weekIndex: number }[] = [];
    let lastMonth = -1;

    grid.forEach((week, i) => {
      const firstDay = new Date(week[0]?.date ?? '');
      const month = firstDay.getMonth();
      if (month !== lastMonth) {
        result.push({
          label: MONTH_LABELS[month],
          weekIndex: i,
        });
        lastMonth = month;
      }
    });

    return result;
  }, [grid]);

  const handleMouseEnter = useCallback((day: DayData, e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top;
    setTooltip({ day, x, y });
  }, []);

  const handleMouseLeave = useCallback(() => {
    setTooltip(null);
  }, []);

  useEffect(() => {
    if (!tooltip || !tooltipRef.current) return;

    const el = tooltipRef.current;
    const rect = el.getBoundingClientRect();
    const margin = 8;

    let x = tooltip.x;
    let y = tooltip.y - 8;
    let translateX = '-50%';
    let translateY = '-100%';

    // Left edge
    if (rect.left < margin) {
      x = margin;
      translateX = '0';
    }
    // Right edge
    if (rect.right > window.innerWidth - margin) {
      x = window.innerWidth - margin;
      translateX = '-100%';
    }
    // Top edge
    if (rect.top < margin) {
      y = tooltip.y + 24;
      translateY = '0';
    }

    setTooltipStyle({
      left: x,
      top: y,
      transform: `translate(${translateX}, ${translateY})`,
    });
  }, [tooltip]);

  const width = grid.length * (CELL_SIZE + CELL_GAP) + 40;
  const height = 7 * (CELL_SIZE + CELL_GAP) + 30;

  return (
    <div className="relative">
      <div className="overflow-x-auto">
        <svg width={width} height={height} className="font-sans text-xs">
          {months.map(({ label, weekIndex }) => (
            <text
              key={`${label}-${weekIndex}`}
              x={weekIndex * (CELL_SIZE + CELL_GAP) + 40}
              y={12}
              fill="currentColor"
              className="fill-muted-foreground"
            >
              {label}
            </text>
          ))}

          {['', 'Mon', '', 'Wed', '', 'Fri', ''].map((label, i) => (
            <text
              key={i}
              x={0}
              y={i * (CELL_SIZE + CELL_GAP) + 28}
              fill="currentColor"
              className="fill-muted-foreground"
            >
              {label}
            </text>
          ))}

          {grid.map((week, weekIndex) =>
            week.map((day, dayIndex) => {
              const intensity = getIntensity(day.tokens);
              return (
                <rect
                  key={day.date}
                  x={weekIndex * (CELL_SIZE + CELL_GAP) + 40}
                  y={dayIndex * (CELL_SIZE + CELL_GAP) + 18}
                  width={CELL_SIZE}
                  height={CELL_SIZE}
                  rx={2}
                  fill={COLORS[intensity]}
                  className="cursor-pointer hover:stroke-2 hover:stroke-foreground"
                  onClick={() => onDayClick?.(day.date)}
                  onMouseEnter={(e) => handleMouseEnter(day, e)}
                  onMouseLeave={handleMouseLeave}
                />
              );
            })
          )}
        </svg>
      </div>

      {tooltip && (
        <div
          ref={tooltipRef}
          className="fixed z-50 bg-white border rounded-md shadow-lg p-3 text-sm pointer-events-none"
          style={tooltipStyle}
        >
          <div className="font-medium mb-1 text-gray-900">{tooltip.day.date}</div>
          <div className="space-y-0.5 text-gray-600">
            <div>Total: <span className="text-gray-900 font-medium">{tooltip.day.tokens.toLocaleString()}</span></div>
            <div>Input: <span className="text-gray-900">{tooltip.day.inputTokens.toLocaleString()}</span></div>
            <div>Cache Hit: <span className="text-gray-900">{tooltip.day.cacheReadTokens.toLocaleString()}</span></div>
            <div>Output: <span className="text-gray-900">{tooltip.day.outputTokens.toLocaleString()}</span></div>
            <div>Cost: <span className="text-gray-900 font-medium">{fmtCost(estimateDayCost(tooltip.day, pricing))}</span></div>
            {tooltip.day.models.length > 0 && (
              <div className="mt-1 pt-1 border-t border-gray-200">
                {tooltip.day.models.slice(0, 3).map((m, i) => (
                  <div key={i} className="text-xs truncate max-w-[200px]">{m}</div>
                ))}
                {tooltip.day.models.length > 3 && (
                  <div className="text-xs">+{tooltip.day.models.length - 3} more</div>
                )}
              </div>
            )}
          </div>
        </div>
      )}

      <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
        <span>Less</span>
        {COLORS.map((color, i) => (
          <div
            key={i}
            className="w-3 h-3 rounded-sm"
            style={{ backgroundColor: color }}
          />
        ))}
        <span>More</span>
      </div>
    </div>
  );
}
