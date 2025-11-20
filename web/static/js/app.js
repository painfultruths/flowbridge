// API Base URL
const API_BASE = '/api';

// State
let tasks = [];
let labels = [];
let draggedTask = null;
let currentTaskId = null;
let confirmCallback = null;
let alertCallback = null;
let preferences = {
    hideCompleted: false,
    confirmDelete: true,
    autoRefresh: 0,
    enableSounds: true,
    theme: 'dark'
};
let autoRefreshInterval = null;

// Time tracking state
let activeTimers = {};
let globalTimerInterval = null;

// Label Picker State
let selectedLabels = {
    newTask: new Set(),
    details: new Set()
};

// New Label Color State
let newLabelColor = {
    newTask: 'gray',
    details: 'gray'
};

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    loadPreferences();
    loadActiveTimers();
    loadTasks();
    loadLabels();
    setupEventListeners();
    setupAutoRefresh();
    startGlobalTimerLoop();
    feather.replace();
});

// Timer Persistence
function loadActiveTimers() {
    const saved = localStorage.getItem('activeTimers');
    if (saved) {
        try {
            activeTimers = JSON.parse(saved);
        } catch (e) {
            console.error('Failed to parse active timers', e);
            activeTimers = {};
        }
    }
}

function saveActiveTimers() {
    localStorage.setItem('activeTimers', JSON.stringify(activeTimers));
}

// Event Listeners
function setupEventListeners() {
    // Header buttons
    document.getElementById('refreshBtn').addEventListener('click', () => {
        loadTasks();
        playSound('action');
    });
    document.getElementById('newTaskBtn').addEventListener('click', showNewTaskDialog);
    document.getElementById('viewArchiveBtn').addEventListener('click', showArchiveDialog);
    document.getElementById('preferencesBtn').addEventListener('click', showPreferencesDialog);

    // New Task dialog buttons
    document.getElementById('createTaskBtn').addEventListener('click', createTask);
    document.getElementById('cancelTaskBtn').addEventListener('click', hideNewTaskDialog);
    document.getElementById('closeNewDialog').addEventListener('click', hideNewTaskDialog);
    document.getElementById('addNewStepBtn').addEventListener('click', () => addNewTaskStepInput());

    // Details dialog buttons
    document.getElementById('saveDetailsBtn').addEventListener('click', saveTaskDetails);
    document.getElementById('closeDetailsBtn').addEventListener('click', hideDetailsDialog);
    document.getElementById('closeDetailsDialog').addEventListener('click', hideDetailsDialog);
    document.getElementById('addCommentBtn').addEventListener('click', addComment);
    document.getElementById('archiveTaskBtn').addEventListener('click', handleArchiveTask);
    document.getElementById('deleteTaskBtn').addEventListener('click', handleDeleteTask);

    // Tab buttons
    document.querySelectorAll('.tab-button').forEach(btn => {
        btn.addEventListener('click', () => switchTab(btn.dataset.tab));
    });

    // Add step button
    document.getElementById('addStepBtn').addEventListener('click', addStep);

    // Time tracking button in details
    document.getElementById('startStopTimer').addEventListener('click', () => {
        if (currentTaskId) {
            toggleCardTimer(currentTaskId);
        }
    });

    // Confirm dialog buttons
    document.getElementById('confirmYesBtn').addEventListener('click', () => {
        hideConfirmDialog();
        if (confirmCallback) {
            confirmCallback(true);
            confirmCallback = null;
        }
    });

    document.getElementById('confirmNoBtn').addEventListener('click', () => {
        hideConfirmDialog();
        if (confirmCallback) {
            confirmCallback(false);
            confirmCallback = null;
        }
    });

    document.getElementById('closeConfirmDialog').addEventListener('click', () => {
        hideConfirmDialog();
        if (confirmCallback) {
            confirmCallback(false);
            confirmCallback = null;
        }
    });

    // Alert dialog buttons
    document.getElementById('alertOkBtn').addEventListener('click', () => {
        hideAlertDialog();
        if (alertCallback) {
            alertCallback();
            alertCallback = null;
        }
    });

    document.getElementById('closeAlertDialog').addEventListener('click', () => {
        hideAlertDialog();
        if (alertCallback) {
            alertCallback();
            alertCallback = null;
        }
    });

    // Click outside to close dialogs
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.addEventListener('click', (e) => {
            if (e.target === overlay) {
                hideNewTaskDialog();
                hideDetailsDialog();
                hideConfirmDialog();
                hideAlertDialog();
                hidePreferencesDialog();
                hideArchiveDialog();
            }
        });
    });

    // Archive dialog
    document.getElementById('closeArchiveBtn').addEventListener('click', hideArchiveDialog);
    document.getElementById('closeArchiveDialog').addEventListener('click', hideArchiveDialog);

    // Preferences dialog
    document.getElementById('savePreferencesBtn').addEventListener('click', savePreferences);
    document.getElementById('cancelPreferencesBtn').addEventListener('click', hidePreferencesDialog);
    document.getElementById('closePreferencesDialog').addEventListener('click', hidePreferencesDialog);
    document.getElementById('clearCompletedBtn').addEventListener('click', clearCompletedTasks);

    // Drag and drop for columns
    const columns = document.querySelectorAll('.column-body');
    columns.forEach(column => {
        column.addEventListener('dragover', handleDragOver);
        column.addEventListener('drop', handleDrop);
        column.addEventListener('dragleave', handleDragLeave);
    });

    // Label Picker Outside Click
    document.addEventListener('click', (e) => {
        if (!e.target.closest('.label-picker')) {
            document.querySelectorAll('.label-picker-dropdown').forEach(d => d.classList.remove('active'));
        }
    });
}

// API Calls
async function loadTasks() {
    try {
        const response = await fetch(`${API_BASE}/tasks`);
        tasks = await response.json();
        renderTasks();
    } catch (error) {
        console.error('Failed to load tasks:', error);
    }
}

async function loadLabels() {
    try {
        const response = await fetch(`${API_BASE}/labels`);
        labels = await response.json();
        // No need to populate select, we render picker dynamically
    } catch (error) {
        console.error('Failed to load labels:', error);
    }
}

// Dynamic Steps Logic
function addNewTaskStepInput(value = '') {
    const container = document.getElementById('newTaskStepsContainer');
    const div = document.createElement('div');
    div.className = 'step-input-group';
    div.innerHTML = `
        <input type="text" class="form-input step-input" placeholder="Enter step..." value="${value}">
        <button class="btn btn-icon" onclick="this.parentElement.remove()">
            <i data-feather="x"></i>
        </button>
    `;
    container.appendChild(div);
    feather.replace();
    div.querySelector('input').focus();
}

// Label Picker Logic
function setupLabelPicker(containerId, context) {
    const container = document.getElementById(containerId);
    if (!container) return;

    container.innerHTML = `
        <div class="label-picker" id="${context}LabelPicker">
            <div class="label-picker-trigger" onclick="toggleLabelDropdown('${context}')">
                <div class="selected-labels" id="${context}SelectedLabels">
                    <span class="text-muted text-sm">Select labels...</span>
                </div>
                <i data-feather="chevron-down"></i>
            </div>
            <div class="label-picker-dropdown" id="${context}LabelDropdown">
                <input type="text" class="label-search" placeholder="Search labels..." onkeyup="filterLabels('${context}', this.value)">
                <div class="label-options" id="${context}LabelOptions"></div>
                <button class="create-label-btn" onclick="showCreateLabelForm('${context}')">
                    <i data-feather="plus" style="width: 12px; height: 12px;"></i> Create new label
                </button>
                <div class="new-label-form hidden" id="${context}NewLabelForm">
                    <input type="text" class="form-input mb-2" id="${context}NewLabelName" placeholder="Label name">
                    <div class="color-swatches" id="${context}ColorSwatches">
                        <!-- Swatches rendered here -->
                    </div>
                    <div class="flex gap-2">
                        <button class="btn btn-primary btn-sm w-full" onclick="createNewLabel('${context}')">Create</button>
                        <button class="btn btn-secondary btn-sm" onclick="hideCreateLabelForm('${context}')">Cancel</button>
                    </div>
                </div>
            </div>
        </div>
    `;
    feather.replace();
    renderLabelOptions(context);
    renderSelectedLabels(context);
    renderColorSwatches(context);
}

function renderColorSwatches(context) {
    const container = document.getElementById(`${context}ColorSwatches`);
    const colors = ['gray', 'red', 'orange', 'yellow', 'green', 'blue', 'purple', 'pink'];

    container.innerHTML = colors.map(color => `
        <div class="color-swatch label-${color} ${newLabelColor[context] === color ? 'selected' : ''}" 
             onclick="selectLabelColor('${context}', '${color}')"></div>
    `).join('');
}

function selectLabelColor(context, color) {
    newLabelColor[context] = color;
    renderColorSwatches(context);
}

function toggleLabelDropdown(context) {
    const dropdown = document.getElementById(`${context}LabelDropdown`);
    const isActive = dropdown.classList.contains('active');

    // Close all others
    document.querySelectorAll('.label-picker-dropdown').forEach(d => d.classList.remove('active'));

    if (!isActive) {
        dropdown.classList.add('active');
        dropdown.querySelector('.label-search').focus();
    }
}

function renderLabelOptions(context) {
    const container = document.getElementById(`${context}LabelOptions`);
    container.innerHTML = '';

    labels.forEach(label => {
        const isSelected = selectedLabels[context].has(label.name);
        const div = document.createElement('div');
        div.className = `label-option ${isSelected ? 'selected' : ''}`;
        div.onclick = () => toggleLabelSelection(context, label.name);
        div.innerHTML = `
            <div class="label-option-color label-${label.color}"></div>
            <span>${label.name}</span>
            <i data-feather="check" class="label-option-check"></i>
        `;
        container.appendChild(div);
    });
    feather.replace();
}

function filterLabels(context, query) {
    const container = document.getElementById(`${context}LabelOptions`);
    const options = container.children;
    query = query.toLowerCase();

    for (let option of options) {
        const text = option.querySelector('span').textContent.toLowerCase();
        if (text.includes(query)) {
            option.classList.remove('hidden');
        } else {
            option.classList.add('hidden');
        }
    }
}

function toggleLabelSelection(context, labelName) {
    if (selectedLabels[context].has(labelName)) {
        selectedLabels[context].delete(labelName);
    } else {
        selectedLabels[context].add(labelName);
    }
    renderLabelOptions(context);
    renderSelectedLabels(context);
}

function renderSelectedLabels(context) {
    const container = document.getElementById(`${context}SelectedLabels`);
    container.innerHTML = '';

    if (selectedLabels[context].size === 0) {
        container.innerHTML = '<span class="text-muted text-sm">Select labels...</span>';
        return;
    }

    selectedLabels[context].forEach(name => {
        const label = labels.find(l => l.name === name);
        if (label) {
            const badge = document.createElement('span');
            badge.className = `label-badge label-${label.color}`;
            badge.textContent = label.name;
            container.appendChild(badge);
        }
    });
}

function showCreateLabelForm(context) {
    document.getElementById(`${context}NewLabelForm`).classList.remove('hidden');
    document.getElementById(`${context}NewLabelName`).focus();
    // Reset color
    newLabelColor[context] = 'gray';
    renderColorSwatches(context);
}

function hideCreateLabelForm(context) {
    document.getElementById(`${context}NewLabelForm`).classList.add('hidden');
}

async function createNewLabel(context) {
    const name = document.getElementById(`${context}NewLabelName`).value.trim();
    const color = newLabelColor[context];

    if (!name) return;

    // Check if exists
    if (labels.some(l => l.name === name)) {
        await showAlert('Label already exists');
        return;
    }

    const newLabel = { name, color };
    labels.push(newLabel);

    // Select it
    selectedLabels[context].add(name);

    // Reset form
    document.getElementById(`${context}NewLabelName`).value = '';
    hideCreateLabelForm(context);

    // Re-render
    renderLabelOptions(context);
    renderSelectedLabels(context);
}

// Make these global for HTML onclick
window.toggleLabelDropdown = toggleLabelDropdown;
window.filterLabels = filterLabels;
window.showCreateLabelForm = showCreateLabelForm;
window.hideCreateLabelForm = hideCreateLabelForm;
window.createNewLabel = createNewLabel;
window.selectLabelColor = selectLabelColor;

async function createTask() {
    const description = document.getElementById('taskDescription').value.trim();
    const details = document.getElementById('taskDetails').value.trim();
    const dueDate = document.getElementById('taskDueDate').value;

    // Collect steps from dynamic inputs
    const steps = Array.from(document.querySelectorAll('#newTaskStepsContainer .step-input'))
        .map(input => input.value.trim())
        .filter(s => s.length > 0);

    if (!description) {
        await showAlert('Please enter a task description');
        playSound('error');
        return;
    }

    // Get selected labels
    const taskLabels = Array.from(selectedLabels.newTask).map(name =>
        labels.find(l => l.name === name)
    ).filter(Boolean);

    const payload = {
        description,
        details: details || null,
        steps: steps.length > 0 ? steps : null,
        due_date: dueDate || null,
        labels: taskLabels.length > 0 ? taskLabels : null
    };

    try {
        const response = await fetch(`${API_BASE}/tasks`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });

        const newTask = await response.json();
        tasks.push(newTask);

        // Reload labels to include any new ones (if backend created them implicitly, though we handle locally)
        await loadLabels();
        renderTasks();
        hideNewTaskDialog();
        playSound('success');

        // Clear form
        document.getElementById('taskDescription').value = '';
        document.getElementById('taskDetails').value = '';
        document.getElementById('newTaskStepsContainer').innerHTML = ''; // Clear steps
        document.getElementById('taskDueDate').value = '';
        selectedLabels.newTask.clear();
        renderSelectedLabels('newTask');
    } catch (error) {
        console.error('Failed to create task:', error);
        await showAlert('Failed to create task');
    }
}

async function updateTaskStatus(taskId, newStatus) {
    try {
        await fetch(`${API_BASE}/tasks/${taskId}/status`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ status: newStatus })
        });

        const task = tasks.find(t => t.id === taskId);
        if (task) {
            task.status = newStatus;
            renderTasks();
            if (newStatus === 'complete') {
                playSound('success');
                triggerConfetti('full');
            } else {
                playSound('action');
            }
        }
    } catch (error) {
        console.error('Failed to update task:', error);
    }
}

async function handleDeleteTask() {
    if (!currentTaskId) return;

    if (preferences.confirmDelete) {
        const confirmed = await showConfirm('Are you sure you want to delete this task? This action cannot be undone.');
        if (!confirmed) {
            return;
        }
    }

    try {
        await fetch(`${API_BASE}/tasks/${currentTaskId}`, {
            method: 'DELETE'
        });

        // Stop timer if running for this task
        if (activeTimers[currentTaskId]) {
            delete activeTimers[currentTaskId];
            saveActiveTimers();
        }

        tasks = tasks.filter(t => t.id !== currentTaskId);
        renderTasks();
        hideDetailsDialog();
        playSound('action');
    } catch (error) {
        console.error('Failed to delete task:', error);
        await showAlert('Failed to delete task');
    }
}

async function saveTaskDetails() {
    if (!currentTaskId) return;

    const description = document.getElementById('detailDescription').value.trim();
    const details = document.getElementById('detailDetails').value.trim();
    const dueDate = document.getElementById('detailDueDate').value;

    // Get selected labels
    const taskLabels = Array.from(selectedLabels.details).map(name =>
        labels.find(l => l.name === name)
    ).filter(Boolean);

    const payload = {
        description: description || undefined,
        details: details || undefined,
        due_date: dueDate || undefined,
        labels: taskLabels.length > 0 ? taskLabels : []
    };

    try {
        await fetch(`${API_BASE}/tasks/${currentTaskId}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });

        // Reload labels to include any new ones
        await loadLabels();
        await loadTasks();
        hideDetailsDialog();
        playSound('success');
    } catch (error) {
        console.error('Failed to save task details:', error);
        await showAlert('Failed to save task details');
    }
}

async function addComment() {
    if (!currentTaskId) return;

    const commentText = document.getElementById('newComment').value.trim();
    if (!commentText) return;

    try {
        await fetch(`${API_BASE}/tasks/${currentTaskId}/comments`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ text: commentText })
        });

        document.getElementById('newComment').value = '';
        await loadTasks();
        const task = tasks.find(t => t.id === currentTaskId);
        if (task) {
            renderComments(task);
            updateCommentTabLabel(task);
        }
        switchTab('comments');
        playSound('action');
    } catch (error) {
        console.error('Failed to add comment:', error);
    }
}

async function toggleStep(stepIndex) {
    if (!currentTaskId) return;

    try {
        await fetch(`${API_BASE}/tasks/${currentTaskId}/toggle-step`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ step_index: stepIndex })
        });

        await loadTasks();
        showTaskDetails(currentTaskId);
        switchTab('steps');
        playSound('action');
    } catch (error) {
        console.error('Failed to toggle step:', error);
    }
}

async function addStep() {
    if (!currentTaskId) return;

    const input = document.getElementById('newStepInput');
    const stepText = input.value.trim();

    if (!stepText) return;

    const task = tasks.find(t => t.id === currentTaskId);
    if (!task) return;

    const newSteps = [...task.steps, { text: stepText, completed: false }];

    try {
        await fetch(`${API_BASE}/tasks/${currentTaskId}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ steps: newSteps })
        });

        input.value = '';
        await loadTasks();
        showTaskDetails(currentTaskId);
        switchTab('steps');
        playSound('action');
    } catch (error) {
        console.error('Failed to add step:', error);
        await showAlert('Failed to add step');
    }
}

async function updateStepText(stepIndex, newText) {
    if (!currentTaskId) return;

    const task = tasks.find(t => t.id === currentTaskId);
    if (!task) return;

    const updatedSteps = task.steps.map((step, idx) =>
        idx === stepIndex ? { ...step, text: newText.trim() } : step
    );

    try {
        await fetch(`${API_BASE}/tasks/${currentTaskId}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ steps: updatedSteps })
        });

        await loadTasks();
        const updatedTask = tasks.find(t => t.id === currentTaskId);
        if (updatedTask) {
            renderSteps(updatedTask);
        }
    } catch (error) {
        console.error('Failed to update step:', error);
    }
}

async function deleteStep(stepIndex) {
    if (!currentTaskId) return;

    if (preferences.confirmDelete) {
        const confirmed = await showConfirm('Delete this step?');
        if (!confirmed) return;
    }

    const task = tasks.find(t => t.id === currentTaskId);
    if (!task) return;

    const updatedSteps = task.steps.filter((_, idx) => idx !== stepIndex);

    try {
        await fetch(`${API_BASE}/tasks/${currentTaskId}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ steps: updatedSteps })
        });

        await loadTasks();
        showTaskDetails(currentTaskId);
        switchTab('steps');
        playSound('action');
    } catch (error) {
        console.error('Failed to delete step:', error);
        await showAlert('Failed to delete step');
    }
}

// Rendering
function renderTasks() {
    const columns = {
        notstarted: document.getElementById('notstarted-column'),
        inprogress: document.getElementById('inprogress-column'),
        inreview: document.getElementById('inreview-column'),
        blocked: document.getElementById('blocked-column'),
        complete: document.getElementById('complete-column')
    };

    // Clear columns
    Object.values(columns).forEach(col => col.innerHTML = '');

    // Group tasks by status
    const grouped = {
        notstarted: [],
        inprogress: [],
        inreview: [],
        blocked: [],
        complete: []
    };

    tasks.forEach(task => {
        // Skip archived tasks
        if (task.archived) {
            return;
        }
        // Skip completed tasks if preference is set
        if (preferences.hideCompleted && task.status === 'complete') {
            return;
        }
        grouped[task.status].push(task);
    });

    // Render tasks in each column
    Object.entries(grouped).forEach(([status, statusTasks]) => {
        const column = columns[status];
        statusTasks.forEach(task => {
            column.appendChild(createTaskCard(task));
        });

        // Update counter
        const countElem = column.closest('.column').querySelector('.column-count');
        countElem.textContent = statusTasks.length;
    });

    // Re-initialize icons
    feather.replace();

    // Force timer update to show correct initial state
    updateAllTimers();
}

function createTaskCard(task) {
    const card = document.createElement('div');
    card.className = 'task-card';
    if (task.status === 'complete') {
        card.classList.add('completed');
    }
    card.draggable = true;
    card.dataset.taskId = task.id;

    // Row 1: Title
    const title = document.createElement('span');
    title.className = 'task-title';
    title.textContent = task.description;
    card.appendChild(title);

    // Row 2: Labels
    if (task.labels && task.labels.length > 0) {
        const labelsDiv = document.createElement('div');
        labelsDiv.className = 'task-labels';
        task.labels.forEach(label => {
            const badge = document.createElement('span');
            const colorClass = label.color ? `label-${label.color}` : 'label-gray';
            badge.className = `label-badge ${colorClass}`;
            badge.textContent = label.name;
            labelsDiv.appendChild(badge);
        });
        card.appendChild(labelsDiv);
    }

    // Row 3: Progress Bar (if steps exist)
    if (task.steps && task.steps.length > 0) {
        const completedCount = task.steps.filter(s => s.completed).length;
        const totalCount = task.steps.length;
        const percentage = Math.round((completedCount / totalCount) * 100);

        const progressWrapper = document.createElement('div');
        progressWrapper.className = 'task-progress-wrapper';

        const progressBar = document.createElement('div');
        progressBar.className = 'task-progress-bar';

        const progressFill = document.createElement('div');
        progressFill.className = 'task-progress-fill';
        progressFill.style.width = `${percentage}%`;

        progressBar.appendChild(progressFill);

        const progressText = document.createElement('div');
        progressText.className = 'task-progress-text';
        progressText.textContent = `${completedCount}/${totalCount}`;

        progressWrapper.appendChild(progressBar);
        progressWrapper.appendChild(progressText);

        card.appendChild(progressWrapper);
    }

    // Row 4: Footer (Date, Comments, Timer)
    const footer = document.createElement('div');
    footer.className = 'task-footer';

    const footerLeft = document.createElement('div');
    footerLeft.className = 'task-footer-left';

    // Due Date
    if (task.due_date) {
        const dueDate = new Date(task.due_date);
        const now = new Date();
        const isOverdue = dueDate < now && task.status !== 'complete';

        const dateBadge = document.createElement('div');
        dateBadge.className = `footer-badge ${isOverdue ? 'overdue' : ''}`;
        dateBadge.innerHTML = `<i data-feather="calendar"></i> ${dueDate.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}`;
        footerLeft.appendChild(dateBadge);
    }

    // Comments
    if (task.comments && task.comments.length > 0) {
        const commentBadge = document.createElement('div');
        commentBadge.className = 'footer-badge';
        commentBadge.innerHTML = `<i data-feather="message-square"></i> ${task.comments.length}`;
        footerLeft.appendChild(commentBadge);
    }

    footer.appendChild(footerLeft);

    // Timer Control
    const timer = document.createElement('div');
    timer.className = 'card-timer';
    if (activeTimers[task.id]) {
        timer.classList.add('active');
    }

    // Initial display calculation
    let totalSeconds = task.time_spent || 0;
    if (activeTimers[task.id]) {
        totalSeconds += Math.floor((Date.now() - activeTimers[task.id]) / 1000);
    }

    timer.innerHTML = `<i data-feather="clock"></i> <span>${formatTime(totalSeconds)}</span>`;
    timer.onclick = (e) => {
        e.stopPropagation();
        toggleCardTimer(task.id);
    };
    footer.appendChild(timer);

    card.appendChild(footer);

    // Click to open details
    card.addEventListener('click', () => showTaskDetails(task.id));

    // Drag events
    card.addEventListener('dragstart', handleDragStart);
    card.addEventListener('dragend', handleDragEnd);

    return card;
}

// Task Details Dialog
function showTaskDetails(taskId) {
    const task = tasks.find(t => t.id === taskId);
    if (!task) return;

    currentTaskId = taskId;

    // Populate general tab
    document.getElementById('detailDescription').value = task.description;
    document.getElementById('detailDetails').value = task.details || '';
    document.getElementById('detailDueDate').value = task.due_date || '';

    // Setup Label Picker for Details
    selectedLabels.details.clear();
    if (task.labels) {
        task.labels.forEach(l => selectedLabels.details.add(l.name));
    }
    setupLabelPicker('detailLabelPickerContainer', 'details');

    // Populate steps tab
    renderSteps(task);

    // Populate comments tab
    renderComments(task);

    // Update comment count in tab
    updateCommentTabLabel(task);

    // Update timer button state
    updateDetailsTimerButton();
    updateDetailsTimeDisplay(task);

    // Show dialog
    document.getElementById('taskDetailsDialog').classList.add('active');
    switchTab('general');
}

function showNewTaskDialog() {
    document.getElementById('newTaskDialog').classList.add('active');
    document.getElementById('taskDescription').focus();

    // Setup Label Picker for New Task
    selectedLabels.newTask.clear();
    setupLabelPicker('taskLabelPickerContainer', 'newTask');

    // Reset steps
    document.getElementById('newTaskStepsContainer').innerHTML = '';
    addNewTaskStepInput(); // Add one empty step by default
}

function updateCommentTabLabel(task) {
    const commentsTab = document.querySelector('.tab-button[data-tab="comments"]');
    const count = task.comments ? task.comments.length : 0;
    commentsTab.textContent = count > 0 ? `Comments (${count})` : 'Comments';
}

function renderSteps(task) {
    const stepsList = document.getElementById('stepsList');
    stepsList.innerHTML = '';

    if (!task.steps || task.steps.length === 0) {
        stepsList.innerHTML = '<p class="text-muted text-sm">No steps defined for this task.</p>';
        return;
    }

    task.steps.forEach((step, index) => {
        const item = document.createElement('div');
        item.className = 'flex items-center gap-2 mb-2';

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = step.completed;
        checkbox.className = 'form-checkbox h-4 w-4 text-indigo-600 transition duration-150 ease-in-out';
        checkbox.addEventListener('change', () => toggleStep(index));

        const input = document.createElement('input');
        input.type = 'text';
        input.value = step.text;
        input.className = 'form-input flex-1 text-sm';
        if (step.completed) {
            input.style.textDecoration = 'line-through';
            input.style.color = 'var(--text-secondary)';
        }
        input.addEventListener('blur', () => updateStepText(index, input.value));
        input.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                input.blur();
            }
        });

        const deleteBtn = document.createElement('button');
        deleteBtn.innerHTML = '&times;';
        deleteBtn.className = 'btn btn-secondary btn-sm';
        deleteBtn.style.padding = '0 0.5rem';
        deleteBtn.addEventListener('click', () => deleteStep(index));

        item.appendChild(checkbox);
        item.appendChild(input);
        item.appendChild(deleteBtn);
        stepsList.appendChild(item);
    });
}

function renderComments(task) {
    const commentsList = document.getElementById('commentsList');
    commentsList.innerHTML = '';

    if (!task.comments || task.comments.length === 0) {
        commentsList.innerHTML = '<p class="text-muted text-sm">No comments yet.</p>';
        return;
    }

    task.comments.forEach(comment => {
        const item = document.createElement('div');
        item.className = 'bg-gray-50 p-3 rounded-md mb-2 border border-gray-200';

        const text = document.createElement('div');
        text.className = 'text-sm mb-1';
        text.innerHTML = linkifyText(comment.text);

        const meta = document.createElement('div');
        meta.className = 'text-xs text-muted';
        meta.textContent = new Date(comment.created_at).toLocaleString();

        item.appendChild(text);
        item.appendChild(meta);
        commentsList.appendChild(item);
    });
}

function linkifyText(text) {
    const urlRegex = /(https?:\/\/[^\s]+)/g;
    return text.replace(urlRegex, '<a href="$1" target="_blank" class="text-indigo-600 hover:underline">$1</a>');
}

function switchTab(tabName) {
    // Update buttons
    document.querySelectorAll('.tab-button').forEach(btn => {
        btn.classList.remove('active');
        if (btn.dataset.tab === tabName) {
            btn.classList.add('active');
        }
    });

    // Update content
    document.querySelectorAll('.tab-content').forEach(content => {
        content.classList.add('hidden');
    });
    document.getElementById(`tab-${tabName}`).classList.remove('hidden');
}

// Drag and Drop
function handleDragStart(e) {
    draggedTask = {
        id: parseInt(e.target.dataset.taskId),
        element: e.target
    };
    e.target.classList.add('dragging');
    e.dataTransfer.effectAllowed = 'move';
}

function handleDragEnd(e) {
    e.target.classList.remove('dragging');
    document.querySelectorAll('.column-body').forEach(col => {
        col.classList.remove('drag-over');
    });
}

function handleDragOver(e) {
    if (e.preventDefault) {
        e.preventDefault();
    }
    e.dataTransfer.dropEffect = 'move';
    // Optional: Add visual cue for drag over
    return false;
}

function handleDragLeave(e) {
    // Optional: Remove visual cue
}

function handleDrop(e) {
    if (e.stopPropagation) {
        e.stopPropagation();
    }
    e.preventDefault();

    const column = e.currentTarget;

    if (draggedTask) {
        const newStatus = column.closest('.column').dataset.status;
        const task = tasks.find(t => t.id === draggedTask.id);

        if (task && task.status !== newStatus) {
            updateTaskStatus(draggedTask.id, newStatus);
        }
    }

    return false;
}

// Custom Dialogs
function showConfirm(message) {
    return new Promise((resolve) => {
        document.getElementById('confirmMessage').textContent = message;
        document.getElementById('confirmDialog').classList.add('active');
        confirmCallback = resolve;
    });
}

function hideConfirmDialog() {
    document.getElementById('confirmDialog').classList.remove('active');
}

function showAlert(message) {
    return new Promise((resolve) => {
        document.getElementById('alertMessage').textContent = message;
        document.getElementById('alertDialog').classList.add('active');
        alertCallback = resolve;
    });
}

function hideAlertDialog() {
    document.getElementById('alertDialog').classList.remove('active');
}

// Dialogs
function hideNewTaskDialog() {
    document.getElementById('newTaskDialog').classList.remove('active');
}

function hideDetailsDialog() {
    document.getElementById('taskDetailsDialog').classList.remove('active');
    currentTaskId = null;
}

// Preferences
function loadPreferences() {
    const saved = localStorage.getItem('taskManagerPreferences');
    if (saved) {
        preferences = { ...preferences, ...JSON.parse(saved) };
    }
    applyTheme();
}

function applyTheme() {
    document.body.dataset.theme = preferences.theme;
}

function showPreferencesDialog() {
    document.getElementById('prefHideCompleted').checked = preferences.hideCompleted;
    document.getElementById('prefConfirmDelete').checked = preferences.confirmDelete;
    document.getElementById('prefAutoRefresh').value = preferences.autoRefresh.toString();
    document.getElementById('prefEnableSounds').checked = preferences.enableSounds;
    document.getElementById('prefTheme').value = preferences.theme;

    document.getElementById('preferencesDialog').classList.add('active');
}

function hidePreferencesDialog() {
    document.getElementById('preferencesDialog').classList.remove('active');
}

function savePreferences() {
    preferences.hideCompleted = document.getElementById('prefHideCompleted').checked;
    preferences.confirmDelete = document.getElementById('prefConfirmDelete').checked;
    preferences.autoRefresh = parseInt(document.getElementById('prefAutoRefresh').value);
    preferences.enableSounds = document.getElementById('prefEnableSounds').checked;
    preferences.theme = document.getElementById('prefTheme').value;

    localStorage.setItem('taskManagerPreferences', JSON.stringify(preferences));

    applyTheme();
    setupAutoRefresh();
    renderTasks();
    hidePreferencesDialog();
    playSound('action');
}

function setupAutoRefresh() {
    if (autoRefreshInterval) {
        clearInterval(autoRefreshInterval);
        autoRefreshInterval = null;
    }

    if (preferences.autoRefresh > 0) {
        autoRefreshInterval = setInterval(() => {
            loadTasks();
        }, preferences.autoRefresh * 1000);
    }
}

async function clearCompletedTasks() {
    const completedTasks = tasks.filter(t => t.status === 'complete');

    if (completedTasks.length === 0) {
        await showAlert('No completed tasks to clear.');
        return;
    }

    const confirmed = await showConfirm(`Delete all ${completedTasks.length} completed task(s)?`);
    if (!confirmed) return;

    try {
        for (const task of completedTasks) {
            await fetch(`${API_BASE}/tasks/${task.id}`, {
                method: 'DELETE'
            });
        }

        await loadTasks();
        await showAlert(`Cleared ${completedTasks.length} completed task(s).`);
        hidePreferencesDialog();
    } catch (error) {
        console.error('Failed to clear completed tasks:', error);
        await showAlert('Failed to clear completed tasks');
    }
}

// Archive Functions
function showArchiveDialog() {
    renderArchiveList();
    document.getElementById('archiveDialog').classList.add('active');
}

function hideArchiveDialog() {
    document.getElementById('archiveDialog').classList.remove('active');
}

function renderArchiveList() {
    const archiveList = document.getElementById('archiveList');
    const archivedTasks = tasks.filter(t => t.archived);

    if (archivedTasks.length === 0) {
        archiveList.innerHTML = '<tr><td colspan="4" style="text-align: center; padding: 20px;" class="text-muted">No archived tasks</td></tr>';
        return;
    }

    archiveList.innerHTML = archivedTasks.map(task => {
        const archivedDate = task.archived_at ? new Date(task.archived_at).toLocaleString() : 'Unknown';
        return `
            <tr class="border-b border-gray-200">
                <td class="p-2">#${task.id}</td>
                <td class="p-2">${escapeHtml(task.description)}</td>
                <td class="p-2 text-sm text-muted">${archivedDate}</td>
                <td class="p-2 text-center">
                    <button onclick="unarchiveTask(${task.id})" class="btn btn-secondary btn-sm mr-2">Unarchive</button>
                    <button onclick="deleteArchivedTask(${task.id})" class="btn btn-danger btn-sm">Delete</button>
                </td>
            </tr>
        `;
    }).join('');
}

async function handleArchiveTask() {
    if (!currentTaskId) return;

    const confirmed = await showConfirm('Archive this task?');
    if (!confirmed) return;

    try {
        const response = await fetch(`${API_BASE}/tasks/${currentTaskId}/archive`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ archived: true })
        });

        if (response.ok) {
            await loadTasks();
            hideDetailsDialog();
            await showAlert('Task archived successfully');
        } else {
            await showAlert('Failed to archive task');
        }
    } catch (error) {
        console.error('Failed to archive task:', error);
        await showAlert('Failed to archive task');
    }
}

// Make these global for onclick handlers in HTML string
window.unarchiveTask = async function (taskId) {
    try {
        const response = await fetch(`${API_BASE}/tasks/${taskId}/archive`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ archived: false })
        });

        if (response.ok) {
            await loadTasks();
            renderArchiveList();
            await showAlert('Task unarchived successfully');
        } else {
            await showAlert('Failed to unarchive task');
        }
    } catch (error) {
        console.error('Failed to unarchive task:', error);
        await showAlert('Failed to unarchive task');
    }
};

window.deleteArchivedTask = async function (taskId) {
    const confirmed = await showConfirm('Permanently delete this archived task?');
    if (!confirmed) return;

    try {
        const response = await fetch(`${API_BASE}/tasks/${taskId}`, {
            method: 'DELETE'
        });

        if (response.ok) {
            await loadTasks();
            renderArchiveList();
            await showAlert('Task deleted permanently');
        } else {
            await showAlert('Failed to delete task');
        }
    } catch (error) {
        console.error('Failed to delete task:', error);
        await showAlert('Failed to delete task');
    }
};

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Time Tracking Functions (Global Loop)
function startGlobalTimerLoop() {
    if (globalTimerInterval) return;

    globalTimerInterval = setInterval(() => {
        updateAllTimers();
    }, 1000);
}

function updateAllTimers() {
    // Update all active timers on cards
    Object.keys(activeTimers).forEach(taskId => {
        const id = parseInt(taskId);
        const task = tasks.find(t => t.id === id);
        if (task) {
            const startTime = activeTimers[id];
            const currentElapsed = Math.floor((Date.now() - startTime) / 1000);
            const totalSeconds = (task.time_spent || 0) + currentElapsed;

            // Update card
            const card = document.querySelector(`.task-card[data-task-id="${id}"]`);
            if (card) {
                const timerSpan = card.querySelector('.card-timer span');
                if (timerSpan) {
                    timerSpan.textContent = formatTime(totalSeconds);
                }
            }

            // Update details dialog if open
            if (currentTaskId === id) {
                const timeDisplay = document.getElementById('timeDisplay');
                if (timeDisplay) {
                    timeDisplay.textContent = formatTime(totalSeconds);
                }
            }
        }
    });
}

function toggleCardTimer(taskId) {
    if (activeTimers[taskId]) {
        stopTimer(taskId);
    } else {
        startTimer(taskId);
    }
}

function startTimer(taskId) {
    const task = tasks.find(t => t.id === taskId);
    if (!task) return;

    activeTimers[taskId] = Date.now();
    saveActiveTimers();

    // Update card visual
    const card = document.querySelector(`.task-card[data-task-id="${taskId}"]`);
    if (card) {
        const timerBtn = card.querySelector('.card-timer');
        if (timerBtn) timerBtn.classList.add('active');
    }

    // Update details button if open
    if (currentTaskId === taskId) {
        updateDetailsTimerButton();
    }
}

async function stopTimer(taskId) {
    if (!activeTimers[taskId]) return;

    const startTime = activeTimers[taskId];
    const currentElapsed = Math.floor((Date.now() - startTime) / 1000);

    delete activeTimers[taskId];
    saveActiveTimers();

    // Update card visual
    const card = document.querySelector(`.task-card[data-task-id="${taskId}"]`);
    if (card) {
        const timerBtn = card.querySelector('.card-timer');
        if (timerBtn) timerBtn.classList.remove('active');
    }

    // Update details button if open
    if (currentTaskId === taskId) {
        updateDetailsTimerButton();
    }

    // Save to backend
    try {
        const task = tasks.find(t => t.id === taskId);
        if (task) {
            const newTotal = (task.time_spent || 0) + currentElapsed;

            await fetch(`${API_BASE}/tasks/${taskId}/time`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ time_spent: newTotal })
            });

            // Update local task
            task.time_spent = newTotal;

            // Update displays
            if (card) {
                const timerSpan = card.querySelector('.card-timer span');
                if (timerSpan) {
                    timerSpan.textContent = formatTime(newTotal);
                }
            }

            if (currentTaskId === taskId) {
                updateDetailsTimeDisplay(task);
            }
        }
    } catch (error) {
        console.error('Failed to save time:', error);
    }
}

function updateDetailsTimerButton() {
    const btn = document.getElementById('startStopTimer');
    if (!btn) return;

    if (activeTimers[currentTaskId]) {
        btn.textContent = 'Stop';
        btn.classList.remove('btn-secondary');
        btn.classList.add('btn-danger');
    } else {
        btn.textContent = 'Start Timer';
        btn.classList.remove('btn-danger');
        btn.classList.add('btn-secondary');
    }
}

function updateDetailsTimeDisplay(task) {
    const timeDisplay = document.getElementById('timeDisplay');
    const totalTimeDisplay = document.getElementById('totalTimeDisplay');

    if (timeDisplay) {
        let totalSeconds = task.time_spent || 0;
        if (activeTimers[task.id]) {
            totalSeconds += Math.floor((Date.now() - activeTimers[task.id]) / 1000);
        }
        timeDisplay.textContent = formatTime(totalSeconds);
    }

    if (totalTimeDisplay) {
        const totalSeconds = task.time_spent || 0;
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        totalTimeDisplay.textContent = `Total: ${hours}h ${minutes}m`;
    }
}

function formatTime(totalSeconds) {
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

// Confetti Effects
function triggerConfetti(type) {
    if (type === 'full') {
        // Big explosion for task completion
        const duration = 3 * 1000;
        const animationEnd = Date.now() + duration;
        const defaults = { startVelocity: 30, spread: 360, ticks: 60, zIndex: 0 };

        const randomInRange = (min, max) => Math.random() * (max - min) + min;

        const interval = setInterval(function () {
            const timeLeft = animationEnd - Date.now();

            if (timeLeft <= 0) {
                return clearInterval(interval);
            }

            const particleCount = 50 * (timeLeft / duration);
            // since particles fall down, start a bit higher than random
            confetti(Object.assign({}, defaults, { particleCount, origin: { x: randomInRange(0.1, 0.3), y: Math.random() - 0.2 } }));
            confetti(Object.assign({}, defaults, { particleCount, origin: { x: randomInRange(0.7, 0.9), y: Math.random() - 0.2 } }));
        }, 250);
    } else if (type === 'mini') {
        // Small burst for steps
        confetti({
            particleCount: 40,
            spread: 70,
            origin: { y: 0.6 },
            colors: ['#a7c080', '#7fbbb3', '#dbbc7f'], // Theme colors
            disableForReducedMotion: true
        });
    }
}

// Sound Effects (Subtle beeps)
function playSound(type) {
    if (!preferences.enableSounds) return;

    const audioContext = new (window.AudioContext || window.webkitAudioContext)();
    const gainNode = audioContext.createGain();
    gainNode.connect(audioContext.destination);

    const now = audioContext.currentTime;

    switch (type) {
        case 'success':
            // Major chord arpeggio for big wins
            [523.25, 659.25, 783.99, 1046.50].forEach((freq, i) => {
                const osc = audioContext.createOscillator();
                osc.type = 'sine';
                osc.frequency.setValueAtTime(freq, now + i * 0.05);

                const oscGain = audioContext.createGain();
                oscGain.gain.setValueAtTime(0.05, now + i * 0.05);
                oscGain.gain.exponentialRampToValueAtTime(0.001, now + i * 0.05 + 0.4);

                osc.connect(oscGain);
                oscGain.connect(audioContext.destination);

                osc.start(now + i * 0.05);
                osc.stop(now + i * 0.05 + 0.4);
            });
            break;

        case 'step':
            // Satisfying high-pitched "chk"
            const osc = audioContext.createOscillator();
            osc.type = 'sine';
            osc.frequency.setValueAtTime(800, now);
            osc.frequency.exponentialRampToValueAtTime(1200, now + 0.1);

            gainNode.gain.setValueAtTime(0.05, now);
            gainNode.gain.exponentialRampToValueAtTime(0.001, now + 0.1);

            osc.connect(gainNode);
            osc.start(now);
            osc.stop(now + 0.1);
            break;

        case 'error':
            const errOsc = audioContext.createOscillator();
            errOsc.type = 'sawtooth';
            errOsc.frequency.setValueAtTime(200, now);
            errOsc.frequency.linearRampToValueAtTime(150, now + 0.2);

            gainNode.gain.setValueAtTime(0.1, now);
            gainNode.gain.exponentialRampToValueAtTime(0.001, now + 0.2);

            errOsc.connect(gainNode);
            errOsc.start(now);
            errOsc.stop(now + 0.2);
            break;

        case 'action':
            const actOsc = audioContext.createOscillator();
            actOsc.frequency.setValueAtTime(400, now);

            gainNode.gain.setValueAtTime(0.03, now);
            gainNode.gain.exponentialRampToValueAtTime(0.001, now + 0.05);

            actOsc.connect(gainNode);
            actOsc.start(now);
            actOsc.stop(now + 0.05);
            break;
    }
}
