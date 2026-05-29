import { useMemo, useState, useRef, useCallback } from 'react';
import type { DailyReport } from '../api/types';

interface ContributionCalendarProps {
  data: Record<string, DailyReport>;
  onDayClick?: (date: string) => void;
}

interface DayData {
  date: string;
  cost: number;
  tokens: number;
  inputTokens: number;
  outputTokens: number;
  models: string[];
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

function getIntensity(cost: number): number {
  if (cost === 0) return 0;
  if (cost < 1) return 1;
  if (cost < 5) return 2;
  if (cost < 20) return 3;
  return 4;
}

export function ContributionCalendar({ data, onDayClick }: ContributionCalendarProps) {
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

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
        r.days?.some(d => d.date === dateStr)
      );
      const day = report?.days?.find(d => d.date === dateStr);

      week.push({
        date: dateStr,
        cost: day?.costUsdNumber ?? 0,
        tokens: day?.totalTokens ?? 0,
        inputTokens: day?.inputTokens ?? 0,
        outputTokens: day?.outputTokens ?? 0,
        models: day?.modelsUsed ?? [],
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
          label: firstDay.toLocaleString('default', { month: 'short' }),
          weekIndex: i,
        });
        lastMonth = month;
      }
    });

    return result;
  }, [grid]);

  const handleMouseEnter = useCallback((day: DayData, weekIndex: number, dayIndex: number) => {
    const x = weekIndex * (CELL_SIZE + CELL_GAP) + 40 + CELL_SIZE / 2;
    const y = dayIndex * (CELL_SIZE + CELL_GAP) + 18;
    setTooltip({ day, x, y });
  }, []);

  const handleMouseLeave = useCallback(() => {
    setTooltip(null);
  }, []);

  const width = grid.length * (CELL_SIZE + CELL_GAP) + 40;
  const height = 7 * (CELL_SIZE + CELL_GAP) + 30;

  return (
    <div className="overflow-x-auto relative" ref={containerRef}>
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
            const intensity = getIntensity(day.cost);
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
                onMouseEnter={() => handleMouseEnter(day, weekIndex, dayIndex)}
                onMouseLeave={handleMouseLeave}
              />
            );
          })
        )}
      </svg>

      {tooltip && (
        <div
          className="absolute z-50 bg-popover border rounded-md shadow-md p-3 text-sm pointer-events-none"
          style={{
            left: tooltip.x,
            top: tooltip.y - 8,
            transform: 'translate(-50%, -100%)',
          }}
        >
          <div className="font-medium mb-1">{tooltip.day.date}</div>
          <div className="space-y-0.5 text-muted-foreground">
            <div>Cost: <span className="text-foreground font-medium">${tooltip.day.cost.toFixed(2)}</span></div>
            <div>Tokens: <span className="text-foreground font-medium">{tooltip.day.tokens.toLocaleString()}</span></div>
            <div>Input: <span className="text-foreground">{tooltip.day.inputTokens.toLocaleString()}</span></div>
            <div>Output: <span className="text-foreground">{tooltip.day.outputTokens.toLocaleString()}</span></div>
            {tooltip.day.models.length > 0 && (
              <div className="mt-1 pt-1 border-t">
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
