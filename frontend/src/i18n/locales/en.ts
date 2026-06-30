// frontend/src/i18n/locales/en.ts
const en = {
  // App header
  'app.title': 'LLM Usage Dashboard',
  'header.updated': 'Updated: ',
  'btn.refresh': 'Refresh',
  'btn.refreshing': 'Refreshing...',
  'btn.backToday': 'Back to today',
  // Source selector
  'source.all': 'All Sources',
  'source.placeholder': 'Select source',
  'source.search': 'Search source...',
  'source.notFound': 'No source found.',
  // Tabs
  'tab.daily': 'Daily',
  'tab.monthly': 'Monthly',
  'tab.session': 'Sessions',
  'tab.blocks': 'Blocks',
  // Chart mode
  'chart.tokenType': 'Token Type',
  'chart.agentSource': 'Agent Source',
  // Metric cards
  'metric.totalTokens': 'Total Tokens',
  'metric.input': 'Input',
  'metric.cacheHit': 'Cache Hit',
  'metric.output': 'Output',
  'metric.cost': 'Est. Cost',
  'metric.costDefault': '(default)',
  // Mosaic / report
  'mosaic.title': 'Usage Mosaic',
  'mosaic.viewing': 'Viewing {date}',
  'report.title': '{tab} Report',
  // Tooltips
  'tooltip.cacheHit': 'Cache hit rate: {rate}%',
  // Status
  'status.noData': 'No data available. Click Refresh to fetch usage data.',
  'status.refreshing': 'Refreshing data in background...',
  // Table headers
  'table.noData': 'No data available',
  'table.date': 'Date',
  'table.month': 'Month',
  'table.total': 'Total',
  'table.input': 'Input',
  'table.cacheHit': 'Cache Hit',
  'table.output': 'Output',
  'table.cost': 'Cost',
  'table.models': 'Models',
  'table.session': 'Session',
  'table.project': 'Project',
  'table.lastActive': 'Last Active',
  'table.status': 'Status',
  'table.startTime': 'Start Time',
  'table.endTime': 'End Time',
  // Block status
  'block.active': 'Active',
  'block.done': 'Done',
  // Pagination
  'page.prev': '← Prev',
  'page.next': 'Next →',
  'page.info': 'Page {current} / {total}',
  // Drill down
  'drill.bySource': 'Drill down by source',
  'drill.byModel': 'Drill down by model',
  'table.reqs': '{n} reqs',
  // Chart bars / tooltip
  'chart.noData': 'No data available',
  'chart.bar.input': 'Input',
  'chart.bar.cacheHit': 'Cache Hit',
  'chart.bar.output': 'Output',
  'chart.bar.cost': 'Est. Cost',
  'chart.tooltip.cost': 'Est. Cost: ',
  // Calendar
  'calendar.less': 'Less',
  'calendar.more': 'More',
  'calendar.moreModels': '+{n} more',
  'cal.total': 'Total: ',
  'cal.input': 'Input: ',
  'cal.cacheHit': 'Cache Hit: ',
  'cal.output': 'Output: ',
  'cal.cost': 'Cost: ',
  // Tray
  'tray.quit': 'Quit',
} as const;

export type TranslationKeys = keyof typeof en;
export default en;
