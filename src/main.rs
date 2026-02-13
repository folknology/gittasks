//! gittask CLI - Git-versioned task management

use anyhow::Result;
use clap::Parser;
use gittask::cli::display::{
    display_stats, display_task_detail, display_task_list, error, success,
};
use gittask::cli::{Cli, Commands};
use gittask::git::GitOperations;
use gittask::models::Task;
use gittask::storage::{FileStore, TaskFilter, TaskLocation};
use std::io::{self, Write};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    let cli = Cli::parse();

    let result = run(cli);

    if let Err(e) = &result {
        error(&e.to_string());
        std::process::exit(1);
    }

    Ok(())
}

fn run(cli: Cli) -> Result<()> {
    let location = if cli.global {
        TaskLocation::global()?
    } else {
        TaskLocation::find_project()?
    };

    match cli.command {
        Commands::Init => {
            if location.exists() {
                log::info!("Task directory already exists: {:?}", location.tasks_dir);
            } else {
                location.ensure_exists()?;
                log::info!("Created task directory: {:?}", location.tasks_dir);
            }
        }

        Commands::Add {
            kind,
            title,
            description,
            priority,
            due,
            tags,
        } => {
            let store = FileStore::new(location.clone());

            if !location.exists() {
                location.ensure_exists()?;
            }

            let mut task = Task::new(0, kind, &title);

            if let Some(desc) = description {
                task.description = desc;
            }

            if let Some(p) = priority {
                task.priority = p;
            }

            task.due = due;
            task.tags = tags;

            let created = store.create(task)?;
            success(&format!(
                "Created {} #{}: {}",
                created.kind, created.id, created.title
            ));
        }

        Commands::List {
            kind,
            status,
            priority,
            tags,
            include_archived,
        } => {
            let store = FileStore::new(location);
            let filter = TaskFilter {
                kind,
                status,
                priority,
                tags,
                include_archived,
            };
            let tasks = store.list(&filter)?;
            display_task_list(&tasks);
        }

        Commands::Show { id } => {
            let store = FileStore::new(location);
            let task = store.read(id)?;
            display_task_detail(&task);
        }

        Commands::Complete { ids } => {
            let store = FileStore::new(location.clone());

            // Get current git commit if available
            let commit = GitOperations::head_commit_optional(&location.root);

            for id in ids {
                let mut task = store.read(id)?;
                task.complete(commit.clone());
                store.update(&task)?;
                success(&format!("Completed #{}: {}", task.id, task.title));
            }
        }

        Commands::Status { id, status } => {
            let store = FileStore::new(location.clone());
            let mut task = store.read(id)?;

            // If completing, capture git commit
            if status == gittask::TaskStatus::Completed
                && task.status != gittask::TaskStatus::Completed
            {
                let commit = GitOperations::head_commit_optional(&location.root);
                task.closed_commit = commit;
            }

            task.status = status;
            task.touch();
            store.update(&task)?;
            success(&format!("Set #{} status to {}", task.id, task.status));
        }

        Commands::Update {
            id,
            title,
            description,
            priority,
            due,
            tags,
        } => {
            let store = FileStore::new(location);
            let mut task = store.read(id)?;

            if let Some(t) = title {
                task.title = t;
            }

            if let Some(d) = description {
                task.description = d;
            }

            if let Some(p) = priority {
                task.priority = p;
            }

            if let Some(d) = due {
                task.due = Some(d);
            }

            if let Some(t) = tags {
                task.tags = t;
            }

            task.touch();
            store.update(&task)?;
            success(&format!("Updated #{}: {}", task.id, task.title));
        }

        Commands::Delete { id, force } => {
            let store = FileStore::new(location);

            if !force {
                let task = store.read(id)?;
                print!("Delete #{} '{}'? [y/N] ", task.id, task.title);
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    log::info!("Cancelled.");
                    return Ok(());
                }
            }

            store.delete(id)?;
            success(&format!("Deleted #{}", id));
        }

        Commands::Stats => {
            let store = FileStore::new(location);
            let stats = store.stats()?;
            display_stats(&stats);
        }
    }

    Ok(())
}
