//! Display formatting for CLI output

use crate::models::Task;
use crate::storage::{AggregatedTask, ProjectStatus, TaskStats};
use tabled::{
    Table, Tabled,
    settings::{Alignment, Modify, Style, object::Columns},
};

/// Task row for table display
#[derive(Tabled)]
struct TaskRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Kind")]
    kind: String,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Priority")]
    priority: String,
    #[tabled(rename = "Due")]
    due: String,
}

impl From<&Task> for TaskRow {
    fn from(task: &Task) -> Self {
        TaskRow {
            id: format!("{}", task.id),
            kind: task.kind.to_string(),
            title: truncate(&task.title, 40),
            status: task.status.to_string(),
            priority: task.priority.to_string(),
            due: task.due.map(|d| d.to_string()).unwrap_or_default(),
        }
    }
}

/// Display a list of tasks as a table
pub fn display_task_list(tasks: &[Task]) {
    if tasks.is_empty() {
        log::info!("No tasks found.");
        return;
    }

    let rows: Vec<TaskRow> = tasks.iter().map(TaskRow::from).collect();
    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::single(0)).with(Alignment::right()))
        .to_string();

    println!("{}", table);
}

/// Aggregated task row for table display (includes project column)
#[derive(Tabled)]
struct AggregatedTaskRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Project")]
    project: String,
    #[tabled(rename = "Kind")]
    kind: String,
    #[tabled(rename = "Title")]
    title: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Priority")]
    priority: String,
    #[tabled(rename = "Due")]
    due: String,
}

impl From<&AggregatedTask> for AggregatedTaskRow {
    fn from(agg: &AggregatedTask) -> Self {
        AggregatedTaskRow {
            id: agg.qualified_id(),
            project: agg.project.clone(),
            kind: agg.task.kind.to_string(),
            title: truncate(&agg.task.title, 35),
            status: agg.task.status.to_string(),
            priority: agg.task.priority.to_string(),
            due: agg.task.due.map(|d| d.to_string()).unwrap_or_default(),
        }
    }
}

/// Display a list of aggregated tasks as a table
pub fn display_aggregated_task_list(tasks: &[AggregatedTask]) {
    if tasks.is_empty() {
        log::info!("No tasks found.");
        return;
    }

    let rows: Vec<AggregatedTaskRow> = tasks.iter().map(AggregatedTaskRow::from).collect();
    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::single(0)).with(Alignment::right()))
        .to_string();

    println!("{}", table);
}

/// Display detailed task information
pub fn display_task_detail(task: &Task) {
    println!("ID:       {}", task.id);
    println!("Title:    {}", task.title);
    println!("Kind:     {}", task.kind);
    println!("Status:   {}", task.status);
    println!("Priority: {}", task.priority);

    if !task.tags.is_empty() {
        println!("Tags:     {}", task.tags.join(", "));
    }

    if let Some(due) = task.due {
        println!("Due:      {}", due);
    }

    println!("Created:  {}", task.created.format("%Y-%m-%d %H:%M:%S"));
    println!("Updated:  {}", task.updated.format("%Y-%m-%d %H:%M:%S"));

    if let Some(ref commit) = task.closed_commit {
        println!("Closed:   {}", commit);
    }

    if !task.description.is_empty() {
        println!();
        println!("Description:");
        println!("{}", task.description);
    }
}

/// Stats row for table display
#[derive(Tabled)]
struct StatsRow {
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Count")]
    count: String,
}

/// Display task statistics
pub fn display_stats(stats: &TaskStats) {
    let rows = vec![
        StatsRow {
            metric: "Total".to_string(),
            count: stats.total.to_string(),
        },
        StatsRow {
            metric: "Pending".to_string(),
            count: stats.pending.to_string(),
        },
        StatsRow {
            metric: "In Progress".to_string(),
            count: stats.in_progress.to_string(),
        },
        StatsRow {
            metric: "Completed".to_string(),
            count: stats.completed.to_string(),
        },
        StatsRow {
            metric: "Archived".to_string(),
            count: stats.archived.to_string(),
        },
        StatsRow {
            metric: "Overdue".to_string(),
            count: stats.overdue.to_string(),
        },
        StatsRow {
            metric: "---".to_string(),
            count: "---".to_string(),
        },
        StatsRow {
            metric: "Tasks".to_string(),
            count: stats.tasks.to_string(),
        },
        StatsRow {
            metric: "Todos".to_string(),
            count: stats.todos.to_string(),
        },
        StatsRow {
            metric: "Ideas".to_string(),
            count: stats.ideas.to_string(),
        },
    ];

    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::single(1)).with(Alignment::right()))
        .to_string();

    println!("{}", table);
}

/// Project row for table display
#[derive(Tabled)]
struct ProjectRow {
    #[tabled(rename = "Project")]
    name: String,
    #[tabled(rename = "Path")]
    path: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Open")]
    open: String,
    #[tabled(rename = "Total")]
    total: String,
}

impl From<&ProjectStatus> for ProjectRow {
    fn from(status: &ProjectStatus) -> Self {
        let status_str = if !status.exists {
            "missing".to_string()
        } else if !status.has_tasks_dir {
            "no .tasks".to_string()
        } else {
            "ok".to_string()
        };

        ProjectRow {
            name: status.name.clone(),
            path: truncate(&status.path.to_string_lossy(), 50),
            status: status_str,
            open: status.open_tasks.to_string(),
            total: status.total_tasks.to_string(),
        }
    }
}

/// Display a list of registered projects
pub fn display_projects(projects: &[ProjectStatus]) {
    if projects.is_empty() {
        log::info!("No projects registered. Use 'gittask link' to register a project.");
        return;
    }

    let rows: Vec<ProjectRow> = projects.iter().map(ProjectRow::from).collect();
    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::new(3..=4)).with(Alignment::right()))
        .to_string();

    println!("{}", table);
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

/// Format for success messages
pub fn success(msg: &str) {
    println!("{}", msg);
}

/// Format for error messages
pub fn error(msg: &str) {
    eprintln!("Error: {}", msg);
}
