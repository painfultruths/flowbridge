use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::Input;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

mod tui;
mod audio;
mod calendar;

#[derive(Parser)]
#[command(name = "task")]
#[command(about = "A tool to help with task initiation and executive dysfunction", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task quickly
    Add {
        /// The task description
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        description: Vec<String>,
    },
    /// Show the next tiny action to start
    Start,
    /// Open kanban board view (TUI)
    Board,
    /// Break down a task into smaller steps
    Break {
        /// Task ID to break down
        id: usize,
    },
    /// Mark a task as done
    Done {
        /// Task ID to complete
        id: usize,
    },
    /// Mark a task as blocked
    Block {
        /// Task ID to block
        id: usize,
    },
    /// Unblock a task
    Unblock {
        /// Task ID to unblock
        id: usize,
    },
    /// Reset a task to Not Started
    Reset {
        /// Task ID to reset
        id: usize,
    },
    /// List all tasks
    List,
    /// Remove a task
    Remove {
        /// Task ID to remove
        id: usize,
    },
    /// Authenticate with Google Calendar
    AuthCalendar,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Blocked,
    Complete,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: usize,
    pub description: String,
    pub steps: Vec<String>,
    pub current_step: usize,
    #[serde(default = "default_status")]
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>, // For backward compatibility
    pub created_at: DateTime<Utc>,
}

fn default_status() -> TaskStatus {
    TaskStatus::NotStarted
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskStore {
    pub tasks: Vec<Task>,
    next_id: usize,
}

impl TaskStore {
    pub fn new() -> Self {
        TaskStore {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            let mut store: TaskStore = serde_json::from_str(&content).unwrap_or_else(|_| Self::new());

            // Migrate old data: convert completed bool to status
            for task in &mut store.tasks {
                if let Some(completed) = task.completed {
                    task.status = if completed {
                        TaskStatus::Complete
                    } else {
                        TaskStatus::NotStarted
                    };
                    task.completed = None;
                }
            }

            store
        } else {
            Self::new()
        }
    }

    pub fn save(&self) {
        let path = Self::get_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let content = serde_json::to_string_pretty(self).unwrap();
        fs::write(&path, content).ok();
    }

    fn get_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".task-data.json")
    }

    pub fn add_task(&mut self, description: String) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(Task {
            id,
            description,
            steps: Vec::new(),
            current_step: 0,
            status: TaskStatus::NotStarted,
            completed: None,
            created_at: Utc::now(),
        });
        id
    }

    pub fn get_task_mut(&mut self, id: usize) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    fn get_next_action(&mut self) -> Option<Task> {
        // Find first non-complete, non-blocked task with steps
        let task_id = {
            if let Some(task) = self.tasks.iter()
                .filter(|t| t.status != TaskStatus::Complete
                         && t.status != TaskStatus::Blocked
                         && !t.steps.is_empty()
                         && t.current_step < t.steps.len())
                .next() {
                Some(task.id)
            } else {
                // Otherwise, find first non-complete, non-blocked task without steps
                self.tasks.iter()
                    .filter(|t| t.status != TaskStatus::Complete
                             && t.status != TaskStatus::Blocked
                             && t.steps.is_empty())
                    .next()
                    .map(|t| t.id)
            }
        };

        if let Some(id) = task_id {
            // Set task to InProgress
            if let Some(task) = self.get_task_mut(id) {
                if task.status == TaskStatus::NotStarted {
                    task.status = TaskStatus::InProgress;
                }
                return Some(task.clone());
            }
        }

        None
    }

    pub fn complete_task(&mut self, id: usize) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            if !task.steps.is_empty() && task.current_step < task.steps.len() - 1 {
                // Move to next step
                task.current_step += 1;
                return true;
            } else {
                // Complete the whole task
                task.status = TaskStatus::Complete;
                return true;
            }
        }
        false
    }

    pub fn block_task(&mut self, id: usize) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            if task.status != TaskStatus::Complete {
                task.status = TaskStatus::Blocked;
                return true;
            }
        }
        false
    }

    pub fn unblock_task(&mut self, id: usize) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            if task.status == TaskStatus::Blocked {
                task.status = if task.current_step > 0 || !task.steps.is_empty() {
                    TaskStatus::InProgress
                } else {
                    TaskStatus::NotStarted
                };
                return true;
            }
        }
        false
    }

    pub fn reset_task(&mut self, id: usize) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            if task.status != TaskStatus::Complete {
                task.status = TaskStatus::NotStarted;
                return true;
            }
        }
        false
    }

    pub fn remove_task(&mut self, id: usize) -> bool {
        let len_before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < len_before
    }
}

fn main() {
    let cli = Cli::parse();
    let mut store = TaskStore::load();

    match cli.command {
        Commands::Add { description } => {
            let desc = description.join(" ");
            if desc.is_empty() {
                eprintln!("{}", "Error: Task description cannot be empty".red());
                std::process::exit(1);
            }
            let id = store.add_task(desc.clone());
            store.save();
            println!("{} Task #{} added: {}", "✓".green(), id, desc);
        }

        Commands::Start => {
            if let Some(task) = store.get_next_action() {
                println!("\n{}", "━".repeat(50).bright_black());
                println!("{}", "NEXT ACTION:".bright_cyan().bold());
                println!("{}", "━".repeat(50).bright_black());

                if task.steps.is_empty() {
                    println!("\n{} {}", "→".bright_yellow(), task.description);
                    println!("\n{}", "This task hasn't been broken down yet.".dimmed());
                    println!("{}", format!("Try: task break {}", task.id).dimmed());
                } else {
                    let current_step = &task.steps[task.current_step];
                    println!("\n{} {}", "→".bright_yellow(), current_step.bold());
                    println!("\n{} {}", "Task:".dimmed(), task.description.dimmed());
                    println!("{} {}/{}", "Step:".dimmed(), task.current_step + 1, task.steps.len());
                    println!("\n{}", format!("When done: task done {}", task.id).bright_green());
                }
                println!("{}\n", "━".repeat(50).bright_black());
            } else {
                println!("{}", "🎉 Nothing to do! Add a task with: task add <description>".bright_green());
            }
        }

        Commands::Board => {
            let mut app = tui::App::new(store);
            match app.run() {
                Ok(_updated_store) => {
                    // Store is already saved by the TUI
                }
                Err(e) => {
                    eprintln!("{}", format!("Error running TUI: {}", e).red());
                    std::process::exit(1);
                }
            }
            return; // Exit after TUI closes
        }

        Commands::Break { id } => {
            // Get task description first
            let task_desc = {
                let task = store.tasks.iter().find(|t| t.id == id);
                if let Some(t) = task {
                    t.description.clone()
                } else {
                    eprintln!("{}", format!("Error: Task #{} not found", id).red());
                    std::process::exit(1);
                }
            };

            println!("\n{}", "Breaking down task:".bright_cyan());
            println!("{}\n", task_desc.bold());

            println!("{}", "Let's break this into tiny, concrete steps.".dimmed());
            println!("{}\n", "Each step should be something you can do in 2-5 minutes.".dimmed());

            let mut steps = Vec::new();
            loop {
                let prompt = if steps.is_empty() {
                    "What's the absolute smallest first action?"
                } else {
                    "Next step? (press Enter to finish)"
                };

                let step: String = Input::new()
                    .with_prompt(prompt)
                    .allow_empty(true)
                    .interact_text()
                    .unwrap();

                if step.is_empty() {
                    if steps.is_empty() {
                        println!("{}", "Need at least one step!".yellow());
                        continue;
                    }
                    break;
                }
                steps.push(step);
            }

            // Now update the task
            let num_steps = steps.len();
            if let Some(task) = store.get_task_mut(id) {
                task.steps = steps;
                task.current_step = 0;
            }
            store.save();

            println!("\n{} Broken into {} steps!", "✓".green(), num_steps);
            println!("{}", format!("Start with: task start").bright_green());
        }

        Commands::Done { id } => {
            if store.complete_task(id) {
                let task = store.tasks.iter().find(|t| t.id == id).unwrap();

                if task.status == TaskStatus::Complete {
                    println!("{} Task #{} completed! 🎉", "✓".green(), id);
                } else {
                    println!("{} Step {} done! Moving to next step.", "✓".green(), task.current_step);
                    println!("{}", format!("Continue with: task start").bright_cyan());
                }
                store.save();
            } else {
                eprintln!("{}", format!("Error: Task #{} not found", id).red());
                std::process::exit(1);
            }
        }

        Commands::Block { id } => {
            if store.block_task(id) {
                store.save();
                println!("{} Task #{} marked as blocked", "⊘".yellow(), id);
                println!("{}", "Task will be skipped by 'task start'".dimmed());
                println!("{}", format!("To unblock: task unblock {}", id).dimmed());
            } else {
                eprintln!("{}", format!("Error: Task #{} not found or already complete", id).red());
                std::process::exit(1);
            }
        }

        Commands::Unblock { id } => {
            if store.unblock_task(id) {
                store.save();
                println!("{} Task #{} unblocked", "✓".green(), id);
            } else {
                eprintln!("{}", format!("Error: Task #{} not found or not blocked", id).red());
                std::process::exit(1);
            }
        }

        Commands::Reset { id } => {
            if store.reset_task(id) {
                store.save();
                println!("{} Task #{} reset to Not Started", "↺".bright_cyan(), id);
            } else {
                eprintln!("{}", format!("Error: Task #{} not found or already complete", id).red());
                std::process::exit(1);
            }
        }

        Commands::List => {
            let incomplete: Vec<_> = store.tasks.iter().filter(|t| t.status != TaskStatus::Complete).collect();

            if incomplete.is_empty() {
                println!("{}", "No active tasks. Add one with: task add <description>".dimmed());
                return;
            }

            println!("\n{}", "ACTIVE TASKS:".bright_cyan().bold());
            println!("{}", "━".repeat(50).bright_black());

            for task in incomplete {
                let status_text = match task.status {
                    TaskStatus::NotStarted => "Not Started".bright_black(),
                    TaskStatus::InProgress => "In Progress".bright_cyan(),
                    TaskStatus::Blocked => "BLOCKED".yellow().bold(),
                    TaskStatus::Complete => "Complete".green(),
                };

                let progress = if task.steps.is_empty() {
                    "not broken down".dimmed()
                } else {
                    format!("step {}/{}", task.current_step + 1, task.steps.len()).dimmed()
                };

                println!("\n#{} {} [{}] {}",
                    task.id.to_string().bright_white().bold(),
                    task.description,
                    status_text,
                    progress
                );

                if !task.steps.is_empty() {
                    for (i, step) in task.steps.iter().enumerate() {
                        let marker = if i < task.current_step {
                            "✓".green()
                        } else if i == task.current_step {
                            "→".bright_yellow()
                        } else {
                            "·".dimmed()
                        };
                        println!("  {} {}", marker, step.dimmed());
                    }
                }
            }
            println!();
        }

        Commands::Remove { id } => {
            if store.remove_task(id) {
                store.save();
                println!("{} Task #{} removed", "✓".green(), id);
            } else {
                eprintln!("{}", format!("Error: Task #{} not found", id).red());
                std::process::exit(1);
            }
        }

        Commands::AuthCalendar => {
            println!("{}", "Setting up Calendar integration (iCal URL)...".bright_cyan());
            println!();
            println!("{}", "To get your iCal URL:".dimmed());
            println!("{}", "  Google Calendar: Settings → Your calendar → Secret address in iCal format".dimmed());
            println!("{}", "  Outlook: Calendar → Share → Publish → Get ICS link".dimmed());
            println!("{}", "  Other: Look for 'iCal', 'webcal', or 'ICS' URL in calendar settings".dimmed());
            println!();

            let url: String = Input::new()
                .with_prompt("Enter your iCal URL")
                .interact_text()
                .unwrap();

            match calendar::save_ical_url(&url) {
                Ok(_) => {
                    println!("{}", "✓ Calendar URL saved!".green());
                    println!("{}", "You can now see your next meeting in the board view.".dimmed());
                }
                Err(e) => {
                    eprintln!("{}", format!("Error: {}", e).red());
                    std::process::exit(1);
                }
            }
        }
    }
}
