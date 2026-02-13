//! gittask CLI - Git-versioned task management

use anyhow::Result;
use clap::Parser;
use gittask::cli::display::{
    display_aggregated_task_list, display_projects, display_stats, display_task_detail,
    display_task_list, error, success,
};
use gittask::cli::{Cli, Commands};
use gittask::git::GitOperations;
use gittask::models::Task;
use gittask::storage::{
    FileStore, ProjectRegistry, TaskFilter, TaskLocation, list_aggregated, resolve_qualified_id,
};
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
            let filter = TaskFilter {
                kind,
                status,
                priority,
                tags,
                include_archived,
            };

            // If global mode and registry has projects, use aggregated view
            if cli.global {
                let registry = ProjectRegistry::load()?;
                if !registry.is_empty() {
                    let tasks = list_aggregated(&registry, &filter)?;
                    display_aggregated_task_list(&tasks);
                    return Ok(());
                }
            }

            // Otherwise, use regular listing
            let store = FileStore::new(location);
            let tasks = store.list(&filter)?;
            display_task_list(&tasks);
        }

        Commands::Show { id } => {
            let registry = ProjectRegistry::load().ok();
            let (resolved_location, task_id) = resolve_qualified_id(
                &id,
                registry.as_ref().unwrap_or(&ProjectRegistry::load()?),
                Some(&location),
            )
            .map_err(|e| anyhow::anyhow!(e))?;

            let store = FileStore::new(resolved_location);
            let task = store.read(task_id)?;
            display_task_detail(&task);
        }

        Commands::Complete { ids } => {
            let registry = ProjectRegistry::load().ok();

            for id_str in ids {
                let (resolved_location, task_id) = resolve_qualified_id(
                    &id_str,
                    registry.as_ref().unwrap_or(&ProjectRegistry::load()?),
                    Some(&location),
                )
                .map_err(|e| anyhow::anyhow!(e))?;

                let store = FileStore::new(resolved_location.clone());

                // Get current git commit from the resolved project
                let commit = GitOperations::head_commit_optional(&resolved_location.root);

                let mut task = store.read(task_id)?;
                task.complete(commit);
                store.update(&task)?;
                success(&format!("Completed #{}: {}", task.id, task.title));
            }
        }

        Commands::Status { id, status } => {
            let registry = ProjectRegistry::load().ok();
            let (resolved_location, task_id) = resolve_qualified_id(
                &id,
                registry.as_ref().unwrap_or(&ProjectRegistry::load()?),
                Some(&location),
            )
            .map_err(|e| anyhow::anyhow!(e))?;

            let store = FileStore::new(resolved_location.clone());
            let mut task = store.read(task_id)?;

            // If completing, capture git commit from the resolved project
            if status == gittask::TaskStatus::Completed
                && task.status != gittask::TaskStatus::Completed
            {
                let commit = GitOperations::head_commit_optional(&resolved_location.root);
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
            let registry = ProjectRegistry::load().ok();
            let (resolved_location, task_id) = resolve_qualified_id(
                &id,
                registry.as_ref().unwrap_or(&ProjectRegistry::load()?),
                Some(&location),
            )
            .map_err(|e| anyhow::anyhow!(e))?;

            let store = FileStore::new(resolved_location);
            let mut task = store.read(task_id)?;

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
            let registry = ProjectRegistry::load().ok();
            let (resolved_location, task_id) = resolve_qualified_id(
                &id,
                registry.as_ref().unwrap_or(&ProjectRegistry::load()?),
                Some(&location),
            )
            .map_err(|e| anyhow::anyhow!(e))?;

            let store = FileStore::new(resolved_location);

            if !force {
                let task = store.read(task_id)?;
                print!("Delete #{} '{}'? [y/N] ", task.id, task.title);
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    log::info!("Cancelled.");
                    return Ok(());
                }
            }

            store.delete(task_id)?;
            success(&format!("Deleted #{}", task_id));
        }

        Commands::Stats => {
            let store = FileStore::new(location);
            let stats = store.stats()?;
            display_stats(&stats);
        }

        Commands::Link { path } => {
            let mut registry = ProjectRegistry::load()?;

            let project_path = if let Some(p) = path {
                p
            } else {
                // Default to current project root
                location.root.clone()
            };

            let inserted = registry.link(&project_path)?;
            if inserted {
                success(&format!("Linked project: {}", project_path.display()));
            } else {
                log::info!("Project already linked: {}", project_path.display());
            }
        }

        Commands::Unlink { path } => {
            let mut registry = ProjectRegistry::load()?;

            let project_path = if let Some(p) = path {
                p
            } else {
                // Default to current project root
                location.root.clone()
            };

            let removed = registry.unlink(&project_path)?;
            if removed {
                success(&format!("Unlinked project: {}", project_path.display()));
            } else {
                log::info!("Project was not linked: {}", project_path.display());
            }
        }

        Commands::Projects => {
            let registry = ProjectRegistry::load()?;
            let statuses = registry.project_statuses();
            display_projects(&statuses);
        }
    }

    Ok(())
}
