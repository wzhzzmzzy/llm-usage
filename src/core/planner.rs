use crate::config::CcUsageConfig;
use crate::core::model::{Cell, ReportType, Source};

/// Represents a command to be executed
#[derive(Debug, Clone)]
pub struct PlannedCommand {
    pub cell: Cell,
    pub program: String,
    pub args: Vec<String>,
}

/// Plans commands for the ccusage provider
pub struct CommandPlanner {
    config: CcUsageConfig,
}

impl CommandPlanner {
    pub fn new(config: CcUsageConfig) -> Self {
        Self { config }
    }

    /// Generate command for a specific cell
    pub fn plan_cell(&self, cell: Cell) -> PlannedCommand {
        let (program, mut args) = if self.config.runner.is_empty() {
            (self.config.package_spec.clone(), Vec::new())
        } else {
            (self.config.runner.clone(), vec![self.config.package_spec.clone()])
        };

        // Add source subcommand if not "all"
        match cell.source {
            Source::All => {}
            Source::Claude => args.push("claude".to_string()),
            Source::Codex => args.push("codex".to_string()),
            Source::Opencode => args.push("opencode".to_string()),
        }

        // Add report type
        args.push(cell.report.as_str().to_string());

        // Add JSON flag
        args.push("--json".to_string());

        // Add extra args
        args.extend(self.config.extra_args.clone());

        PlannedCommand {
            cell,
            program,
            args,
        }
    }

    /// Generate all commands for the matrix (16 cells)
    pub fn plan_all(&self) -> Vec<PlannedCommand> {
        Cell::all_cells().into_iter().map(|c| self.plan_cell(c)).collect()
    }

    /// Generate commands for a specific source
    pub fn plan_source(&self, source: Source) -> Vec<PlannedCommand> {
        ReportType::all_variants()
            .iter()
            .map(|report| {
                self.plan_cell(Cell {
                    source,
                    report: *report,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CcUsageConfig {
        CcUsageConfig {
            runner: "bunx".to_string(),
            package_spec: "ccusage".to_string(),
            extra_args: vec![],
        }
    }

    #[test]
    fn test_plan_cell_all_daily() {
        let planner = CommandPlanner::new(test_config());
        let cmd = planner.plan_cell(Cell {
            source: Source::All,
            report: ReportType::Daily,
        });
        assert_eq!(cmd.program, "bunx");
        assert_eq!(cmd.args, vec!["ccusage", "daily", "--json"]);
    }

    #[test]
    fn test_plan_cell_claude_monthly() {
        let planner = CommandPlanner::new(test_config());
        let cmd = planner.plan_cell(Cell {
            source: Source::Claude,
            report: ReportType::Monthly,
        });
        assert_eq!(cmd.program, "bunx");
        assert_eq!(cmd.args, vec!["ccusage", "claude", "monthly", "--json"]);
    }

    #[test]
    fn test_plan_cell_codex_session() {
        let planner = CommandPlanner::new(test_config());
        let cmd = planner.plan_cell(Cell {
            source: Source::Codex,
            report: ReportType::Session,
        });
        assert_eq!(cmd.program, "bunx");
        assert_eq!(cmd.args, vec!["ccusage", "codex", "session", "--json"]);
    }

    #[test]
    fn test_plan_cell_opencode_blocks() {
        let planner = CommandPlanner::new(test_config());
        let cmd = planner.plan_cell(Cell {
            source: Source::Opencode,
            report: ReportType::Blocks,
        });
        assert_eq!(cmd.program, "bunx");
        assert_eq!(cmd.args, vec!["ccusage", "opencode", "blocks", "--json"]);
    }

    #[test]
    fn test_plan_all_returns_16_commands() {
        let planner = CommandPlanner::new(test_config());
        let cmds = planner.plan_all();
        assert_eq!(cmds.len(), 16);
    }

    #[test]
    fn test_plan_source_claude_returns_4_commands() {
        let planner = CommandPlanner::new(test_config());
        let cmds = planner.plan_source(Source::Claude);
        assert_eq!(cmds.len(), 4);
        for cmd in &cmds {
            assert!(cmd.args.contains(&"claude".to_string()));
        }
    }

    #[test]
    fn test_version_pinning() {
        let config = CcUsageConfig {
            runner: "bunx".to_string(),
            package_spec: "ccusage@15.10.0".to_string(),
            extra_args: vec![],
        };
        let planner = CommandPlanner::new(config);
        let cmd = planner.plan_cell(Cell {
            source: Source::All,
            report: ReportType::Daily,
        });
        assert_eq!(cmd.args[0], "ccusage@15.10.0");
    }

    #[test]
    fn test_extra_args() {
        let config = CcUsageConfig {
            runner: "bunx".to_string(),
            package_spec: "ccusage".to_string(),
            extra_args: vec!["--verbose".to_string()],
        };
        let planner = CommandPlanner::new(config);
        let cmd = planner.plan_cell(Cell {
            source: Source::All,
            report: ReportType::Daily,
        });
        assert_eq!(cmd.args, vec!["ccusage", "daily", "--json", "--verbose"]);
    }
}
