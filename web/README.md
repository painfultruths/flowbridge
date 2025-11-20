# Task Manager - Windows 98 Edition

A beautiful, nostalgic task management web app with an authentic Windows 98 aesthetic. Built for people with executive dysfunction who need help breaking down and tracking tasks.

## ✨ Features

- **Authentic Windows 98 UI** - Pixel-perfect recreation of the classic Windows 98 interface
- **Kanban Board** - Visual task management with drag-and-drop between 5 columns
- **Task Breakdown** - Add, edit, and delete steps to break down complex tasks
- **Comments System** - Add comments with automatic URL linking
- **Custom Labels** - Create reusable labels with 8 preset colors
- **Due Dates** - Track deadlines with overdue warnings
- **Persistent Storage** - All data saved to disk (survives restarts!)
- **Auto-Refresh** - Optional automatic task reloading
- **Preferences** - Customizable settings (hide completed, confirmations, etc.)
- **Classic Design** - Dog-eared sticky note cards, beveled buttons, and classic scrollbars
- **Zero Dependencies UI** - Pure HTML/CSS/JS with no frameworks needed

## 🚀 Quick Start with Docker

```bash
# Using Docker Compose (recommended)
docker-compose up

# Or using Docker directly
docker build -t task-manager-98 .
docker run -p 3000:3000 task-manager-98
```

Then open http://localhost:3000 in your browser!

## 💾 Data Persistence

All your tasks, steps, comments, and labels are automatically saved to disk. Your data persists across:
- Application restarts
- Docker container restarts
- System reboots

### Docker Volume

When using docker-compose, a named volume `task-data` is automatically created to store your data.

**Backup your data:**
```bash
# Find the container name
docker ps

# Copy data file
docker cp <container-name>:/app/data/tasks.json ./backup.json
```

**Restore data:**
```bash
docker cp ./backup.json <container-name>:/app/data/tasks.json
docker restart <container-name>
```

### Local Development

Data is stored in `./data/tasks.json` by default. To use a custom location:

```bash
DATA_DIR=/path/to/data ./target/release/task-web
```

## 🛠️ Development

### Prerequisites
- Rust 1.75 or later
- Node.js (optional, for development)

### Running Locally

```bash
# Build the backend
cargo build --release

# Run the server
cargo run

# The app will be available at http://localhost:3000
```

### Project Structure

```
web/
├── src/
│   └── main.rs          # Rust backend API server
├── static/
│   ├── index.html       # Main HTML
│   ├── css/
│   │   └── win98.css    # Windows 98 styles
│   └── js/
│       └── app.js       # Frontend JavaScript
├── Cargo.toml
├── Dockerfile
└── docker-compose.yml
```

## 📋 API Endpoints

- `GET /api/tasks` - List all tasks
- `GET /api/labels` - List all labels
- `POST /api/tasks` - Create a new task
- `PUT /api/tasks/:id` - Update task (description, details, label, due date, steps)
- `PUT /api/tasks/:id/status` - Update task status
- `DELETE /api/tasks/:id` - Delete a task
- `POST /api/tasks/:id/comments` - Add a comment
- `POST /api/tasks/:id/toggle-step` - Toggle step completion

## 🎨 Design Philosophy

This project recreates the Windows 98 aesthetic with:
- Authentic color palette (#C0C0C0 gray, #000080 title bar blue)
- 3D beveled UI elements
- Classic system fonts (MS Sans Serif)
- Pixel-perfect window chrome
- Nostalgic scrollbars and buttons
- Dog-eared sticky note cards for tasks

## 🏗️ Tech Stack

- **Backend**: Rust + Axum (lightweight, fast web framework)
- **Frontend**: Vanilla HTML/CSS/JavaScript
- **Styling**: Custom CSS recreating Windows 98 UI
- **Deployment**: Docker + Docker Compose

## 📸 Screenshot-Worthy Features

- Clean kanban board with 4 columns (Not Started, In Progress, Blocked, Complete)
- Draggable task cards with folded corner effect
- Classic Windows 98 title bars and window chrome
- Authentic menu bar and toolbar
- Working clock in the taskbar
- Modal dialogs with proper Windows 98 styling

## 🤝 Contributing

Feel free to open issues or submit PRs to improve this nostalgic productivity tool!

## 📜 License

MIT License - Feel free to use this for your own projects!

---

**Made with ❤️ for people who need a little help getting started** 🚀
