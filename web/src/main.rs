use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::PathBuf;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TaskStatus {
    NotStarted,
    InProgress,
    InReview,
    Blocked,
    Complete,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Step {
    text: String,
    completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Comment {
    text: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Label {
    name: String,
    color: String, // red, orange, yellow, green, blue, purple, pink, gray
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: usize,
    description: String,
    details: Option<String>,
    steps: Vec<Step>,
    comments: Vec<Comment>,
    status: TaskStatus,
    labels: Vec<Label>,
    due_date: Option<String>, // Store as YYYY-MM-DD string
    created_at: DateTime<Utc>,
    archived: bool,
    archived_at: Option<DateTime<Utc>>,
    time_spent: u64, // Time spent in seconds
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskStore {
    tasks: Vec<Task>,
    labels: Vec<Label>,
    next_id: usize,
    #[serde(skip)]
    data_file: Option<PathBuf>,
}

impl TaskStore {
    fn new() -> Self {
        TaskStore {
            tasks: Vec::new(),
            labels: Vec::new(),
            next_id: 1,
            data_file: None,
        }
    }

    fn load_from_file(path: PathBuf) -> Self {
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(contents) => {
                    match serde_json::from_str::<TaskStore>(&contents) {
                        Ok(mut store) => {
                            println!("✅ Loaded {} tasks from {:?}", store.tasks.len(), &path);
                            store.data_file = Some(path);
                            return store;
                        }
                        Err(e) => {
                            eprintln!("⚠️  Failed to parse data file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to read data file: {}", e);
                }
            }
        }

        let mut store = Self::new();
        store.data_file = Some(path);
        store
    }

    fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = &self.data_file {
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    fn get_or_add_label(&mut self, label: Label) -> Label {
        // If label exists, return it; otherwise add and return
        if let Some(existing) = self.labels.iter().find(|l| l.name == label.name) {
            existing.clone()
        } else {
            self.labels.push(label.clone());
            let _ = self.save_to_file();
            label
        }
    }

    fn add_task(&mut self, description: String, details: Option<String>, due_date: Option<String>, labels: Vec<Label>) -> Task {
        let id = self.next_id;
        self.next_id += 1;
        let task = Task {
            id,
            description,
            details,
            steps: Vec::new(),
            comments: Vec::new(),
            status: TaskStatus::NotStarted,
            labels,
            due_date,
            created_at: Utc::now(),
            archived: false,
            archived_at: None,
            time_spent: 0,
        };
        self.tasks.push(task.clone());
        let _ = self.save_to_file();
        task
    }

    fn get_task_mut(&mut self, id: usize) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    fn remove_task(&mut self, id: usize) -> bool {
        let len_before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        let removed = self.tasks.len() < len_before;
        if removed {
            let _ = self.save_to_file();
        }
        removed
    }
}

type SharedState = Arc<Mutex<TaskStore>>;

#[derive(Deserialize)]
struct CreateTaskRequest {
    description: String,
    details: Option<String>,
    steps: Option<Vec<String>>,
    due_date: Option<String>,
    labels: Option<Vec<Label>>,
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: TaskStatus,
}

#[derive(Deserialize)]
struct UpdateTaskRequest {
    description: Option<String>,
    details: Option<String>,
    labels: Option<Vec<Label>>,
    due_date: Option<String>,
    steps: Option<Vec<Step>>,
}

#[derive(Deserialize)]
struct AddCommentRequest {
    text: String,
}

#[derive(Deserialize)]
struct ToggleStepRequest {
    step_index: usize,
}

#[derive(Deserialize)]
struct ArchiveTaskRequest {
    archived: bool,
}

#[derive(Deserialize)]
struct UpdateTimeRequest {
    time_spent: u64,
}

async fn list_tasks(State(state): State<SharedState>) -> Json<Vec<Task>> {
    let store = state.lock().unwrap();
    Json(store.tasks.clone())
}

async fn list_labels(State(state): State<SharedState>) -> Json<Vec<Label>> {
    let store = state.lock().unwrap();
    Json(store.labels.clone())
}

async fn create_task(
    State(state): State<SharedState>,
    Json(req): Json<CreateTaskRequest>,
) -> (StatusCode, Json<Task>) {
    let mut store = state.lock().unwrap();

    // If labels are provided, add them to global labels if not exists
    let labels = if let Some(lbls) = req.labels {
        lbls.into_iter().map(|l| store.get_or_add_label(l)).collect()
    } else {
        Vec::new()
    };

    let mut task = store.add_task(req.description, req.details, req.due_date, labels);
    if let Some(steps) = req.steps {
        let step_structs: Vec<Step> = steps.into_iter().map(|text| Step { text, completed: false }).collect();
        if let Some(t) = store.get_task_mut(task.id) {
            t.steps = step_structs.clone();
            task.steps = step_structs;
        }
    }
    (StatusCode::CREATED, Json(task))
}

async fn update_task_status(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
    Json(req): Json<UpdateStatusRequest>,
) -> StatusCode {
    let mut store = state.lock().unwrap();
    if let Some(task) = store.get_task_mut(id) {
        task.status = req.status;
        let _ = store.save_to_file();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn delete_task(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
) -> StatusCode {
    let mut store = state.lock().unwrap();
    if store.remove_task(id) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn update_task(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
    Json(req): Json<UpdateTaskRequest>,
) -> StatusCode {
    let mut store = state.lock().unwrap();

    // Process labels first to avoid borrow checker issues
    let saved_labels = if let Some(labels) = req.labels {
        Some(labels.into_iter().map(|l| store.get_or_add_label(l)).collect::<Vec<Label>>())
    } else {
        None
    };

    if let Some(task) = store.get_task_mut(id) {
        if let Some(description) = req.description {
            task.description = description;
        }
        if let Some(details) = req.details {
            task.details = Some(details);
        }
        if let Some(labels) = saved_labels {
            task.labels = labels;
        }
        if let Some(due_date) = req.due_date {
            task.due_date = Some(due_date);
        }
        if let Some(steps) = req.steps {
            task.steps = steps;
        }
        let _ = store.save_to_file();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn add_comment(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
    Json(req): Json<AddCommentRequest>,
) -> StatusCode {
    let mut store = state.lock().unwrap();
    if let Some(task) = store.get_task_mut(id) {
        task.comments.push(Comment {
            text: req.text,
            created_at: Utc::now(),
        });
        let _ = store.save_to_file();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn toggle_step(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
    Json(req): Json<ToggleStepRequest>,
) -> StatusCode {
    let mut store = state.lock().unwrap();
    if let Some(task) = store.get_task_mut(id) {
        if req.step_index < task.steps.len() {
            task.steps[req.step_index].completed = !task.steps[req.step_index].completed;
            let _ = store.save_to_file();
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        }
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn archive_task(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
    Json(req): Json<ArchiveTaskRequest>,
) -> StatusCode {
    let mut store = state.lock().unwrap();
    if let Some(task) = store.get_task_mut(id) {
        task.archived = req.archived;
        task.archived_at = if req.archived {
            Some(Utc::now())
        } else {
            None
        };
        let _ = store.save_to_file();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn update_time(
    State(state): State<SharedState>,
    Path(id): Path<usize>,
    Json(req): Json<UpdateTimeRequest>,
) -> StatusCode {
    let mut store = state.lock().unwrap();
    if let Some(task) = store.get_task_mut(id) {
        task.time_spent = req.time_spent;
        let _ = store.save_to_file();
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

#[tokio::main]
async fn main() {
    // Get data directory from env or use default
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let data_path = PathBuf::from(&data_dir);

    // Create data directory if it doesn't exist
    if !data_path.exists() {
        fs::create_dir_all(&data_path).expect("Failed to create data directory");
    }

    let data_file = data_path.join("tasks.json");
    let state = Arc::new(Mutex::new(TaskStore::load_from_file(data_file)));

    let app = Router::new()
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/:id/status", put(update_task_status))
        .route("/api/tasks/:id", put(update_task).delete(delete_task))
        .route("/api/tasks/:id/comments", post(add_comment))
        .route("/api/tasks/:id/toggle-step", post(toggle_step))
        .route("/api/tasks/:id/archive", put(archive_task))
        .route("/api/tasks/:id/time", put(update_time))
        .route("/api/labels", get(list_labels))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .nest_service("/", ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("🚀 Task Manager running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
