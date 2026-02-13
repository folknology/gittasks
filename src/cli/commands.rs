//! CLI command definitions using clap

use crate::models::{Priority, TaskKind, TaskStatus};
use chrono::NaiveDate;
use clap::{Parser, Subcommand};

/// Git-versioned task management using Markdown files
#[derive(Parser, Debug)]
#[command(name = "gittask")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Use global tasks directory (~/.tasks) instead of project-local
    #[arg(short, long, global = true)]
    pub global: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize the .tasks directory
    Init,

    /// Add a new task
    Add {
        /// Task kind (task, todo, idea)
        #[arg(value_parser = parse_kind)]
        kind: TaskKind,

        /// Task title
        title: String,

        /// Task description
        #[arg(short, long)]
        description: Option<String>,

        /// Priority (low, medium, high, critical)
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,

        /// Due date (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date)]
        due: Option<NaiveDate>,

        /// Tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,
    },

    /// List tasks
    List {
        /// Filter by kind
        #[arg(short, long, value_parser = parse_kind)]
        kind: Option<TaskKind>,

        /// Filter by status
        #[arg(short, long, value_parser = parse_status)]
        status: Option<TaskStatus>,

        /// Filter by priority
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,

        /// Filter by tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Include archived tasks
        #[arg(short = 'a', long)]
        include_archived: bool,
    },

    /// Show task details
    Show {
        /// Task ID
        id: u64,
    },

    /// Mark task(s) as completed
    Complete {
        /// Task ID(s)
        ids: Vec<u64>,
    },

    /// Change task status
    Status {
        /// Task ID
        id: u64,

        /// New status (pending, in-progress, completed, archived)
        #[arg(value_parser = parse_status)]
        status: TaskStatus,
    },

    /// Update task properties
    Update {
        /// Task ID
        id: u64,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New description
        #[arg(short, long)]
        description: Option<String>,

        /// New priority
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,

        /// New due date (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date)]
        due: Option<NaiveDate>,

        /// New tags (comma-separated, replaces existing)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },

    /// Delete a task
    Delete {
        /// Task ID
        id: u64,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show task statistics
    Stats,
}

fn parse_kind(s: &str) -> Result<TaskKind, String> {
    s.parse()
}

fn parse_status(s: &str) -> Result<TaskStatus, String> {
    s.parse()
}

fn parse_priority(s: &str) -> Result<Priority, String> {
    s.parse()
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| format!("Invalid date: {}", e))
}
