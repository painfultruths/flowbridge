use crate::{Task, TaskStatus, TaskStore};
use chrono::{Local, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

#[derive(PartialEq)]
enum AppMode {
    Navigate,
    AddTask,
    EditStep,
    EditTaskName,
    ConfirmDelete,
}

#[derive(Default)]
struct TaskForm {
    description: String,
    steps: Vec<String>,
    current_step_input: String,
    active_field: usize, // 0 = description, 1 = step input, 2 = submit
}

pub struct App {
    store: TaskStore,
    mode: AppMode,
    selected_column: usize,
    selected_task: Option<usize>,
    should_quit: bool,
    form: TaskForm,
    edit_buffer: String,
    editing_task_id: Option<usize>,
    deleting_task_id: Option<usize>,
    column_areas: Vec<Rect>,
    dragging_task: Option<(usize, usize)>, // (task_id, original_column)
    drag_target_column: Option<usize>,
    next_meeting: Option<crate::calendar::NextMeeting>,
}

impl App {
    pub fn new(store: TaskStore) -> Self {
        // Fetch next meeting
        let next_meeting = crate::calendar::get_next_meeting_sync();

        App {
            store,
            mode: AppMode::Navigate,
            selected_column: 0,
            selected_task: None,
            should_quit: false,
            form: TaskForm::default(),
            edit_buffer: String::new(),
            editing_task_id: None,
            deleting_task_id: None,
            column_areas: Vec::new(),
            dragging_task: None,
            drag_target_column: None,
            next_meeting,
        }
    }

    pub fn run(&mut self) -> io::Result<TaskStore> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main loop
        while !self.should_quit {
            terminal.draw(|f| self.ui(f))?;
            self.handle_events()?;
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(std::mem::replace(&mut self.store, TaskStore::new()))
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match self.mode {
                            AppMode::Navigate => self.handle_navigate_keys(key.code),
                            AppMode::AddTask => self.handle_form_keys(key.code),
                            AppMode::EditStep => self.handle_edit_keys(key.code),
                            AppMode::EditTaskName => self.handle_edit_task_name_keys(key.code),
                            AppMode::ConfirmDelete => self.handle_confirm_keys(key.code),
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    if self.mode == AppMode::Navigate {
                        self.handle_mouse(mouse);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_navigate_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('a') => {
                self.mode = AppMode::AddTask;
                self.form = TaskForm::default();
            }
            KeyCode::Left => {
                if self.selected_column > 0 {
                    self.selected_column -= 1;
                    self.selected_task = None;
                }
            }
            KeyCode::Right => {
                if self.selected_column < 3 {
                    self.selected_column += 1;
                    self.selected_task = None;
                }
            }
            KeyCode::Up => self.select_previous_task(),
            KeyCode::Down => self.select_next_task(),
            KeyCode::Char('n') => self.move_to_not_started(),
            KeyCode::Char('i') => self.move_to_in_progress(),
            KeyCode::Char('b') => self.move_to_blocked(),
            KeyCode::Char('d') | KeyCode::Char(' ') => self.complete_task(),
            KeyCode::Char('u') => self.undo_step(),
            KeyCode::Char('e') => self.start_edit_step(),
            KeyCode::Char('E') => self.start_edit_task_name(),
            KeyCode::Char('r') => self.remove_task(),
            _ => {}
        }
    }

    fn handle_edit_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Navigate;
                self.edit_buffer.clear();
                self.editing_task_id = None;
            }
            KeyCode::Enter => {
                self.save_edited_step();
            }
            KeyCode::Char(c) => {
                self.edit_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.edit_buffer.pop();
            }
            _ => {}
        }
    }

    fn handle_confirm_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Confirm delete
                if let Some(id) = self.deleting_task_id {
                    self.store.remove_task(id);
                    self.store.save();
                    self.selected_task = None;
                }
                self.mode = AppMode::Navigate;
                self.deleting_task_id = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                // Cancel delete
                self.mode = AppMode::Navigate;
                self.deleting_task_id = None;
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        let x = mouse.column;
        let y = mouse.row;

        match mouse.kind {
            MouseEventKind::Down(event::MouseButton::Left) => {
                // Start drag operation
                // Check which column and task was clicked
                for (col_idx, area) in self.column_areas.iter().enumerate() {
                    if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
                        self.selected_column = col_idx;

                        // Determine which task was clicked
                        let relative_y = y.saturating_sub(area.y + 1);

                        let status = match col_idx {
                            0 => TaskStatus::NotStarted,
                            1 => TaskStatus::InProgress,
                            2 => TaskStatus::Blocked,
                            _ => TaskStatus::Complete,
                        };
                        let tasks = self.get_tasks_by_status(status);

                        // Calculate which task based on card positions
                        let mut current_line = 0;
                        let mut drag_info = None;
                        for (task_idx, task) in tasks.iter().enumerate() {
                            let card_height = if task.steps.is_empty() { 3 } else { 4 };
                            if relative_y >= current_line && relative_y < current_line + card_height {
                                drag_info = Some((task_idx, task.id));
                                break;
                            }
                            current_line += card_height;
                        }

                        // Now update state
                        if let Some((task_idx, task_id)) = drag_info {
                            self.selected_task = Some(task_idx);
                            self.dragging_task = Some((task_id, col_idx));
                            self.drag_target_column = Some(col_idx);
                        }

                        break;
                    }
                }
            }
            MouseEventKind::Drag(event::MouseButton::Left) => {
                // Update drag target column based on mouse position
                if self.dragging_task.is_some() {
                    for (col_idx, area) in self.column_areas.iter().enumerate() {
                        if x >= area.x && x < area.x + area.width {
                            self.drag_target_column = Some(col_idx);
                            break;
                        }
                    }
                }
            }
            MouseEventKind::Up(event::MouseButton::Left) => {
                // Complete drag-and-drop
                if let Some((task_id, original_col)) = self.dragging_task {
                    if let Some(target_col) = self.drag_target_column {
                        if target_col != original_col {
                            // Move task to new status
                            let new_status = match target_col {
                                0 => TaskStatus::NotStarted,
                                1 => TaskStatus::InProgress,
                                2 => TaskStatus::Blocked,
                                _ => TaskStatus::Complete,
                            };

                            let is_complete = new_status == TaskStatus::Complete;

                            if let Some(task) = self.store.get_task_mut(task_id) {
                                task.status = new_status;
                                self.store.save();

                                // Play chime if moved to Complete
                                if is_complete {
                                    crate::audio::play_completion_chime();
                                }
                            }

                            // Update selection to new column
                            self.selected_column = target_col;
                            self.selected_task = None;
                        }
                    }
                }
                // Clear drag state
                self.dragging_task = None;
                self.drag_target_column = None;
            }
            _ => {}
        }
    }

    fn handle_form_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Navigate;
                self.form = TaskForm::default();
            }
            KeyCode::Tab => {
                // Cycle through: description (0) -> step input (1) -> submit (2) -> back to 0
                self.form.active_field = (self.form.active_field + 1) % 3;
            }
            KeyCode::Enter => {
                match self.form.active_field {
                    0 => {
                        // On description field, move to step input
                        self.form.active_field = 1;
                    }
                    1 => {
                        // On step input, add the step and stay on this field
                        if !self.form.current_step_input.is_empty() {
                            self.form.steps.push(self.form.current_step_input.clone());
                            self.form.current_step_input.clear();
                            // Stay on field 1 so they can keep adding steps
                        }
                    }
                    2 => {
                        // On submit button
                        self.submit_task();
                    }
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                match self.form.active_field {
                    0 => self.form.description.push(c),
                    1 => self.form.current_step_input.push(c),
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                match self.form.active_field {
                    0 => { self.form.description.pop(); }
                    1 => { self.form.current_step_input.pop(); }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn submit_task(&mut self) {
        if !self.form.description.is_empty() {
            let id = self.store.add_task(self.form.description.clone());

            // Add steps if any
            if !self.form.steps.is_empty() {
                if let Some(task) = self.store.get_task_mut(id) {
                    task.steps = self.form.steps.clone();
                }
            }

            self.store.save();
            self.mode = AppMode::Navigate;
            self.form = TaskForm::default();
        }
    }

    fn get_tasks_by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.store
            .tasks
            .iter()
            .filter(|t| t.status == status)
            .collect()
    }

    fn select_next_task(&mut self) {
        let tasks = self.get_tasks_by_status(self.current_status());
        if tasks.is_empty() {
            return;
        }

        self.selected_task = Some(match self.selected_task {
            None => 0,
            Some(i) if i >= tasks.len() - 1 => tasks.len() - 1,
            Some(i) => i + 1,
        });
    }

    fn select_previous_task(&mut self) {
        let tasks = self.get_tasks_by_status(self.current_status());
        if tasks.is_empty() {
            return;
        }

        self.selected_task = Some(match self.selected_task {
            None => 0,
            Some(0) => 0,
            Some(i) => i - 1,
        });
    }

    fn current_status(&self) -> TaskStatus {
        match self.selected_column {
            0 => TaskStatus::NotStarted,
            1 => TaskStatus::InProgress,
            2 => TaskStatus::Blocked,
            _ => TaskStatus::Complete,
        }
    }

    fn get_selected_task_id(&self) -> Option<usize> {
        let tasks = self.get_tasks_by_status(self.current_status());
        self.selected_task.and_then(|idx| tasks.get(idx).map(|t| t.id))
    }

    fn move_to_not_started(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            self.store.reset_task(id);
            self.store.save();
            self.selected_task = None;
        }
    }

    fn move_to_in_progress(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            if let Some(task) = self.store.get_task_mut(id) {
                task.status = TaskStatus::InProgress;
                self.store.save();
                self.selected_task = None;
            }
        }
    }

    fn move_to_blocked(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            self.store.block_task(id);
            self.store.save();
            self.selected_task = None;
        }
    }

    fn complete_task(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            self.store.complete_task(id);
            self.store.save();

            // Only deselect if the task is now complete (moved to Complete column)
            // Otherwise keep it selected so user can see the next step
            if let Some(task) = self.store.tasks.iter().find(|t| t.id == id) {
                if task.status == TaskStatus::Complete {
                    self.selected_task = None;
                    // Play completion chime!
                    crate::audio::play_completion_chime();
                }
            }
        }
    }

    fn remove_task(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            self.deleting_task_id = Some(id);
            self.mode = AppMode::ConfirmDelete;
        }
    }

    fn undo_step(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            if let Some(task) = self.store.get_task_mut(id) {
                if task.current_step > 0 {
                    task.current_step -= 1;
                    self.store.save();
                }
            }
        }
    }

    fn start_edit_step(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            if let Some(task) = self.store.tasks.iter().find(|t| t.id == id) {
                if !task.steps.is_empty() && task.current_step < task.steps.len() {
                    self.edit_buffer = task.steps[task.current_step].clone();
                    self.editing_task_id = Some(id);
                    self.mode = AppMode::EditStep;
                }
            }
        }
    }

    fn save_edited_step(&mut self) {
        if let Some(id) = self.editing_task_id {
            if let Some(task) = self.store.get_task_mut(id) {
                if !self.edit_buffer.is_empty() && task.current_step < task.steps.len() {
                    task.steps[task.current_step] = self.edit_buffer.clone();
                    self.store.save();
                }
            }
        }
        self.mode = AppMode::Navigate;
        self.edit_buffer.clear();
        self.editing_task_id = None;
    }

    fn start_edit_task_name(&mut self) {
        if let Some(id) = self.get_selected_task_id() {
            if let Some(task) = self.store.tasks.iter().find(|t| t.id == id) {
                self.edit_buffer = task.description.clone();
                self.editing_task_id = Some(id);
                self.mode = AppMode::EditTaskName;
            }
        }
    }

    fn handle_edit_task_name_keys(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Navigate;
                self.edit_buffer.clear();
                self.editing_task_id = None;
            }
            KeyCode::Enter => {
                self.save_edited_task_name();
            }
            KeyCode::Char(c) => {
                self.edit_buffer.push(c);
            }
            KeyCode::Backspace => {
                self.edit_buffer.pop();
            }
            _ => {}
        }
    }

    fn save_edited_task_name(&mut self) {
        if let Some(id) = self.editing_task_id {
            if let Some(task) = self.store.get_task_mut(id) {
                if !self.edit_buffer.is_empty() {
                    task.description = self.edit_buffer.clone();
                    self.store.save();
                }
            }
        }
        self.mode = AppMode::Navigate;
        self.edit_buffer.clear();
        self.editing_task_id = None;
    }

    fn ui(&mut self, f: &mut Frame) {
        // Main horizontal split: Left panel (33%) | Right kanban (67%)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
            .split(f.area());

        // Left panel vertical split: Clock/Message | Meeting | Form/Details
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),  // Clock/Message
                Constraint::Length(4),        // Meeting info
                Constraint::Min(10),          // Form/Details (takes remaining space)
            ])
            .split(main_chunks[0]);

        // Right panel vertical split: Kanban board | Help
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(3)])
            .split(main_chunks[1]);

        // Render left side
        self.render_clock_panel(f, left_chunks[0]);
        self.render_meeting_panel(f, left_chunks[1]);

        match self.mode {
            AppMode::Navigate => self.render_task_details(f, left_chunks[2]),
            AppMode::AddTask => self.render_task_form(f, left_chunks[2]),
            AppMode::EditStep => self.render_edit_step(f, left_chunks[2]),
            AppMode::EditTaskName => self.render_edit_task_name(f, left_chunks[2]),
            AppMode::ConfirmDelete => self.render_confirm_delete(f, left_chunks[2]),
        }

        // Render right side - Kanban board
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(right_chunks[0]);

        // Store column areas for mouse support
        self.column_areas = columns.to_vec();

        self.render_column(f, columns[0], "Not Started (n)", TaskStatus::NotStarted, Color::Gray, 0);
        self.render_column(f, columns[1], "In Progress (i)", TaskStatus::InProgress, Color::Cyan, 1);
        self.render_column(f, columns[2], "Blocked (b)", TaskStatus::Blocked, Color::Yellow, 2);
        self.render_column(f, columns[3], "Complete", TaskStatus::Complete, Color::Green, 3);

        // Help text
        let help_text = match self.mode {
            AppMode::Navigate => "a: Add | SPACE/d: Done | u: Undo | e: Edit Step | E: Edit Name | ←/→: Columns | ↑/↓: Tasks | r: Remove | Drag & Drop: Move Cards | q: Quit",
            AppMode::AddTask => "Tab: Next Field | Enter: Add Step/Submit | ESC: Cancel",
            AppMode::EditStep => "Type to edit step | Enter: Save | ESC: Cancel",
            AppMode::EditTaskName => "Type to edit task name | Enter: Save | ESC: Cancel",
            AppMode::ConfirmDelete => "y: Yes, delete | n: No, cancel | ESC: Cancel",
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, right_chunks[1]);
    }

    fn get_ascii_digit(digit: char) -> [&'static str; 5] {
        // Using Unicode box-drawing and block elements for a cleaner look
        match digit {
            '0' => [
                "╔═══╗",
                "║   ║",
                "║   ║",
                "║   ║",
                "╚═══╝",
            ],
            '1' => [
                "  ╔═╗",
                "  ║ ║",
                "  ║ ║",
                "  ║ ║",
                "  ╚═╝",
            ],
            '2' => [
                "╔═══╗",
                "    ║",
                "╔═══╝",
                "║    ",
                "╚═══╗",
            ],
            '3' => [
                "╔═══╗",
                "    ║",
                " ═══╣",
                "    ║",
                "╚═══╝",
            ],
            '4' => [
                "╔   ║",
                "║   ║",
                "╚═══╣",
                "    ║",
                "    ╚",
            ],
            '5' => [
                "╔═══╗",
                "║    ",
                "╚═══╗",
                "    ║",
                "╚═══╝",
            ],
            '6' => [
                "╔═══╗",
                "║    ",
                "╠═══╗",
                "║   ║",
                "╚═══╝",
            ],
            '7' => [
                "╔═══╗",
                "    ║",
                "    ║",
                "    ║",
                "    ╚",
            ],
            '8' => [
                "╔═══╗",
                "║   ║",
                "╠═══╣",
                "║   ║",
                "╚═══╝",
            ],
            '9' => [
                "╔═══╗",
                "║   ║",
                "╚═══╣",
                "    ║",
                "╚═══╝",
            ],
            ':' => [
                "     ",
                "  ●  ",
                "     ",
                "  ●  ",
                "     ",
            ],
            ' ' => [
                "     ",
                "     ",
                "     ",
                "     ",
                "     ",
            ],
            _ => [
                "╔═══╗",
                "║░░░║",
                "║░░░║",
                "║░░░║",
                "╚═══╝",
            ],
        }
    }

    fn render_clock_panel(&self, f: &mut Frame, area: Rect) {
        let now = Local::now();
        let time_str = now.format("%I:%M").to_string();
        let ampm_str = now.format("%p").to_string();

        let messages = vec![
            "You've got this! 💪",
            "One small step at a time",
            "Progress over perfection",
            "Your brain is doing its best",
            "Take it easy on yourself",
            "Small wins count too",
            "You're showing up - that matters",
            "Breaking tasks down is smart",
            "It's okay to go slow",
            "Every step forward counts",
        ];

        // Rotate message every 5 minutes (300 seconds)
        let message = messages[(now.timestamp() / 300) as usize % messages.len()];

        // Build Unicode clock
        let chars: Vec<char> = time_str.chars().collect();
        let mut ascii_lines = vec![String::new(); 5];

        for ch in chars {
            let digit_lines = Self::get_ascii_digit(ch);
            for i in 0..5 {
                ascii_lines[i].push_str(digit_lines[i]);
                ascii_lines[i].push(' '); // Space between digits
            }
        }

        let mut content = vec![Line::from("")];

        // Add ASCII clock lines
        for line in &ascii_lines {
            content.push(Line::from(Span::styled(
                line,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        content.push(Line::from(Span::styled(
            ampm_str,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            message,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        )));

        let panel = Paragraph::new(content)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(panel, area);
    }

    fn render_meeting_panel(&self, f: &mut Frame, area: Rect) {
        use chrono::Local;

        let content = if let Some(ref meeting) = self.next_meeting {
            let now = Local::now();
            let start_local = meeting.start_time.with_timezone(&Local::now().timezone());

            // Calculate time until meeting
            let duration = meeting.start_time.signed_duration_since(now.with_timezone(&Utc));

            let time_str = if duration.num_minutes() < 0 {
                "Now".to_string()
            } else if duration.num_hours() < 1 {
                format!("in {} min", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("in {}h {}m", duration.num_hours(), duration.num_minutes() % 60)
            } else {
                format!("in {} days", duration.num_days())
            };

            let time_display = start_local.format("%I:%M %p").to_string();

            vec![
                Line::from(vec![
                    Span::styled("Next: ", Style::default().fg(Color::Yellow)),
                    Span::styled(&meeting.summary, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled(format!("{} ", time_display), Style::default().fg(Color::Cyan)),
                    Span::styled(format!("({})", time_str), Style::default().fg(Color::DarkGray)),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "No upcoming meetings",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                )),
            ]
        };

        let panel = Paragraph::new(content)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(panel, area);
    }

    fn render_task_form(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                "Add New Task",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        // Task description field
        let desc_style = if self.form.active_field == 0 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(Span::styled(
            "Task Description:",
            Style::default().fg(Color::DarkGray),
        )));

        let cursor = if self.form.active_field == 0 { "█" } else { "" };
        lines.push(Line::from(Span::styled(
            format!("> {}{}", self.form.description, cursor),
            desc_style,
        )));
        lines.push(Line::from(""));

        // Steps section
        lines.push(Line::from(Span::styled(
            "Break it down into smaller steps:",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
        lines.push(Line::from(Span::styled(
            "(helps with executive dysfunction!)",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Existing steps
        for (i, step) in self.form.steps.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Green)),
                Span::styled(step, Style::default().fg(Color::White)),
            ]));
        }

        // Current step input
        let step_style = if self.form.active_field == 1 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let step_cursor = if self.form.active_field == 1 {
            "█"
        } else {
            ""
        };

        lines.push(Line::from(Span::styled(
            format!("> {}{}", self.form.current_step_input, step_cursor),
            step_style,
        )));
        lines.push(Line::from(Span::styled(
            "(Press Enter to add step, Tab to submit)",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Submit button
        let submit_style = if self.form.active_field == 2 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        lines.push(Line::from(Span::styled("[ Create Task ]", submit_style)));

        let form_panel = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" New Task Form ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(form_panel, area);
    }

    fn render_edit_step(&self, f: &mut Frame, area: Rect) {
        let task_info = if let Some(id) = self.editing_task_id {
            self.store.tasks.iter()
                .find(|t| t.id == id)
                .map(|t| (t.description.clone(), t.current_step + 1, t.steps.len()))
        } else {
            None
        };

        let mut lines = vec![
            Line::from(Span::styled(
                "Edit Step",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if let Some((desc, step_num, total_steps)) = task_info {
            lines.push(Line::from(vec![
                Span::styled("Task: ", Style::default().fg(Color::DarkGray)),
                Span::styled(desc, Style::default().fg(Color::Cyan)),
            ]));
            lines.push(Line::from(Span::styled(
                format!("Step {}/{}", step_num, total_steps),
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "Edit step description:",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            format!("> {}█", self.edit_buffer),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "Press Enter to save",
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(Span::styled(
            "Press ESC to cancel",
            Style::default().fg(Color::DarkGray),
        )));

        let edit_panel = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Edit Step ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(edit_panel, area);
    }

    fn render_edit_task_name(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                "Edit Task Name",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(""),
        ];

        lines.push(Line::from(Span::styled(
            "Edit task description:",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            format!("> {}█", self.edit_buffer),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "Press Enter to save",
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(Span::styled(
            "Press ESC to cancel",
            Style::default().fg(Color::DarkGray),
        )));

        let edit_panel = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Edit Task Name ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(edit_panel, area);
    }

    fn render_confirm_delete(&self, f: &mut Frame, area: Rect) {
        let task_desc = if let Some(id) = self.deleting_task_id {
            self.store.tasks.iter()
                .find(|t| t.id == id)
                .map(|t| t.description.clone())
        } else {
            None
        };

        let mut lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "⚠ DELETE TASK?",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(""),
        ];

        if let Some(desc) = task_desc {
            lines.push(Line::from(Span::styled(
                "Are you sure you want to delete:",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("\"{}\"", desc),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(Span::styled(
            "This cannot be undone!",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::ITALIC),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("[", Style::default().fg(Color::DarkGray)),
            Span::styled(" Y ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("] Yes, delete    [", Style::default().fg(Color::DarkGray)),
            Span::styled(" N ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("] No, keep it", Style::default().fg(Color::DarkGray)),
        ]));

        let confirm_panel = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" ⚠ CONFIRM DELETE ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(confirm_panel, area);
    }

    fn render_column(
        &mut self,
        f: &mut Frame,
        area: Rect,
        title: &str,
        status: TaskStatus,
        color: Color,
        column_idx: usize,
    ) {
        let tasks = self.get_tasks_by_status(status);
        let is_selected_column = self.selected_column == column_idx;
        let is_drag_target = self.drag_target_column == Some(column_idx);

        // Column border style
        let border_style = if is_drag_target {
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        } else if is_selected_column {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Render column container
        let column_block = Block::default()
            .title(format!(" {} ({}) ", title, tasks.len()))
            .borders(Borders::ALL)
            .border_style(border_style);

        f.render_widget(column_block, area);

        // Area inside the column border for cards
        let inner_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        // Render each task as a card
        let mut current_y = inner_area.y;
        for (idx, task) in tasks.iter().enumerate() {
            let is_task_selected = is_selected_column && self.selected_task == Some(idx);
            let is_being_dragged = self.dragging_task.map(|(id, _)| id == task.id).unwrap_or(false);

            // Card height: 3 lines (1 for description, 1 for progress, 1 for border spacing)
            let card_height = if task.steps.is_empty() { 3 } else { 4 };

            // Stop rendering if we run out of space
            if current_y + card_height > inner_area.y + inner_area.height {
                break;
            }

            let card_area = Rect {
                x: inner_area.x,
                y: current_y,
                width: inner_area.width,
                height: card_height,
            };

            // Card border style (more subtle selection)
            let border_color = if is_being_dragged {
                Color::Magenta
            } else if is_task_selected {
                color  // Use column color without bold
            } else {
                color
            };

            let bg_color = if is_task_selected {
                Some(Color::DarkGray)
            } else {
                None
            };

            // Render card with dog ear
            self.render_card(
                f,
                card_area,
                task,
                border_color,
                bg_color,
            );

            current_y += card_height;
        }
    }

    fn render_card(
        &self,
        f: &mut Frame,
        area: Rect,
        task: &Task,
        border_color: Color,
        bg_color: Option<Color>,
    ) {
        let has_steps = !task.steps.is_empty();

        // Build card with manual borders and dog ear
        let mut lines = Vec::new();

        // Top border
        let top_border = format!("╭{}╮", "─".repeat(area.width.saturating_sub(2) as usize));
        lines.push(Line::from(Span::styled(top_border, Style::default().fg(border_color))));

        // Content line: task description
        let desc_text = format!("#{} {}", task.id, task.description);
        let desc_truncated = if desc_text.len() > (area.width.saturating_sub(4) as usize) {
            format!("{}…", &desc_text[..area.width.saturating_sub(5) as usize])
        } else {
            desc_text
        };
        let padding = area.width.saturating_sub(desc_truncated.len() as u16 + 2);

        let content_spans = vec![
            Span::styled("│", Style::default().fg(border_color)),
            Span::styled(format!("{}{}", desc_truncated, " ".repeat(padding as usize)),
                Style::default().fg(Color::White).bg(bg_color.unwrap_or(Color::Black))),
            Span::styled("│", Style::default().fg(border_color)),
        ];
        lines.push(Line::from(content_spans));

        // Optional steps line
        if has_steps {
            let step_text = format!("  step {}/{}", task.current_step + 1, task.steps.len());
            let step_padding = area.width.saturating_sub(step_text.len() as u16 + 2);
            lines.push(Line::from(vec![
                Span::styled("│", Style::default().fg(border_color)),
                Span::styled(format!("{}{}", step_text, " ".repeat(step_padding as usize)),
                    Style::default().fg(Color::DarkGray).bg(bg_color.unwrap_or(Color::Black))),
                Span::styled("│", Style::default().fg(border_color)),
            ]));
        }

        // Bottom border with dog ear - simple triangle fold in bottom-right
        let bottom_width = area.width.saturating_sub(3) as usize;
        let bottom_border = format!("╰{}◣", "─".repeat(bottom_width));
        lines.push(Line::from(Span::styled(bottom_border, Style::default().fg(border_color))));

        let card = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(card, area);
    }

    fn render_task_details(&self, f: &mut Frame, area: Rect) {
        // Get the currently selected task
        let task = if let Some(task_id) = self.get_selected_task_id() {
            self.store.tasks.iter().find(|t| t.id == task_id)
        } else {
            None
        };

        let content = if let Some(task) = task {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Task #", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{}", task.id), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw(": "),
                    Span::styled(&task.description, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
            ];

            if task.steps.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No steps defined. Use 'task break <id>' to break this down.",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                )));
            } else {
                // Progress indicator
                lines.push(Line::from(Span::styled(
                    format!("Progress: {}/{} steps complete", task.current_step, task.steps.len()),
                    Style::default().fg(Color::Cyan),
                )));
                lines.push(Line::from(""));

                // Completed steps
                if task.current_step > 0 {
                    lines.push(Line::from(Span::styled(
                        "✓ Completed:",
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )));
                    for i in 0..task.current_step {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("✓ ", Style::default().fg(Color::Green)),
                            Span::styled(&task.steps[i], Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                    lines.push(Line::from(""));
                }

                // Current step - HIGHLIGHTED
                if task.current_step < task.steps.len() {
                    lines.push(Line::from(Span::styled(
                        "▶ DO THIS NOW:",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));

                    // Big highlighted box for current step
                    let current_step_text = &task.steps[task.current_step];
                    lines.push(Line::from(Span::styled(
                        "┌────────────────────────────┐",
                        Style::default().fg(Color::Yellow),
                    )));
                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{:<26}", current_step_text.chars().take(26).collect::<String>()),
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" │", Style::default().fg(Color::Yellow)),
                    ]));
                    lines.push(Line::from(Span::styled(
                        "└────────────────────────────┘",
                        Style::default().fg(Color::Yellow),
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("SPACE", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled("/", Style::default().fg(Color::DarkGray)),
                        Span::styled("d", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(": Complete | ", Style::default().fg(Color::DarkGray)),
                        Span::styled("u", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(": Undo | ", Style::default().fg(Color::DarkGray)),
                        Span::styled("e", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(": Edit", Style::default().fg(Color::DarkGray)),
                    ]));
                    lines.push(Line::from(""));
                }

                // Upcoming steps
                if task.current_step < task.steps.len() - 1 {
                    lines.push(Line::from(Span::styled(
                        "Next steps:",
                        Style::default().fg(Color::DarkGray),
                    )));
                    for i in (task.current_step + 1)..task.steps.len() {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("· ", Style::default().fg(Color::DarkGray)),
                            Span::styled(&task.steps[i], Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }

            Text::from(lines)
        } else {
            Text::from(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No task selected",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Use ↑/↓ to select a task",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "Use ←/→ to switch columns",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        };

        let border_color = if task.is_some() {
            Color::Yellow  // Highlighted when task selected
        } else {
            Color::DarkGray
        };

        let details = Paragraph::new(content)
            .block(
                Block::default()
                    .title(" Task Details ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(details, area);
    }
}
