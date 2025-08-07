/**
 * Translation Project Kanban Board
 * Provides drag-and-drop task management with real-time updates
 */

class KanbanBoard {
    constructor() {
        this.projectId = this.getProjectIdFromUrl();
        this.board = null;
        this.cards = [];
        this.teamMembers = [];
        this.currentEditingCard = null;
        this.eventSource = null;
        this.searchTerm = '';
        this.filters = {
            assignee: '',
            priority: ''
        };

        this.init();
    }

    async init() {
        this.setupEventListeners();
        this.setupDragAndDrop();
        await this.loadBoard();
        this.setupRealTimeUpdates();
    }

    getProjectIdFromUrl() {
        const urlParams = new URLSearchParams(window.location.search);
        return urlParams.get('project') || 'default-project-id';
    }

    setupEventListeners() {
        // Header controls
        document.getElementById('add-task-btn').addEventListener('click', () => this.openTaskModal());
        document.getElementById('refresh-btn').addEventListener('click', () => this.loadBoard());
        document.getElementById('search-input').addEventListener('input', (e) => this.handleSearch(e.target.value));
        document.getElementById('assignee-filter').addEventListener('change', (e) => this.handleFilter('assignee', e.target.value));
        document.getElementById('priority-filter').addEventListener('change', (e) => this.handleFilter('priority', e.target.value));

        // Modal controls
        document.getElementById('close-modal').addEventListener('click', () => this.closeTaskModal());
        document.getElementById('cancel-btn').addEventListener('click', () => this.closeTaskModal());
        document.getElementById('task-form').addEventListener('submit', (e) => this.handleTaskSubmit(e));

        // Close modal on outside click
        document.getElementById('task-modal').addEventListener('click', (e) => {
            if (e.target.id === 'task-modal') {
                this.closeTaskModal();
            }
        });

        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeTaskModal();
            }
            if (e.key === 'n' && (e.ctrlKey || e.metaKey)) {
                e.preventDefault();
                this.openTaskModal();
            }
        });
    }

    setupDragAndDrop() {
        // Will be set up after board is loaded
    }

    async loadBoard() {
        this.showLoading(true);
        try {
            const response = await fetch(`/api/projects/${this.projectId}/kanban`);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            
            this.board = await response.json();
            await this.loadTeamMembers();
            this.renderBoard();
            this.showToast('Board loaded successfully', 'success');
        } catch (error) {
            console.error('Error loading board:', error);
            this.showToast('Failed to load board', 'error');
        } finally {
            this.showLoading(false);
        }
    }

    async loadTeamMembers() {
        try {
            const response = await fetch(`/api/projects/${this.projectId}/members`);
            if (response.ok) {
                this.teamMembers = await response.json();
                this.populateAssigneeOptions();
            }
        } catch (error) {
            console.error('Error loading team members:', error);
        }
    }

    populateAssigneeOptions() {
        const assigneeFilter = document.getElementById('assignee-filter');
        const taskAssignee = document.getElementById('task-assignee');

        // Clear existing options (except "All Assignees" for filter)
        assigneeFilter.innerHTML = '<option value="">All Assignees</option>';
        taskAssignee.innerHTML = '<option value="">Unassigned</option>';

        this.teamMembers.forEach(member => {
            const filterOption = document.createElement('option');
            filterOption.value = member.user_id;
            filterOption.textContent = member.name;
            assigneeFilter.appendChild(filterOption);

            const taskOption = document.createElement('option');
            taskOption.value = member.user_id;
            taskOption.textContent = member.name;
            taskAssignee.appendChild(taskOption);
        });
    }

    renderBoard() {
        const boardContainer = document.getElementById('kanban-board');
        const projectTitle = document.getElementById('project-title');
        
        projectTitle.textContent = this.board.project_name || 'Translation Project Kanban';
        boardContainer.innerHTML = '';

        // Define column order and titles
        const columnConfig = [
            { status: 'todo', title: 'To Do', color: '#6c757d' },
            { status: 'in_progress', title: 'In Progress', color: '#007bff' },
            { status: 'review', title: 'Review', color: '#ffc107' },
            { status: 'done', title: 'Done', color: '#28a745' }
        ];

        columnConfig.forEach(config => {
            const column = this.createColumn(config);
            boardContainer.appendChild(column);
        });

        this.setupColumnDragAndDrop();
    }

    createColumn(config) {
        const cards = this.board.columns[config.status] || [];
        const filteredCards = this.filterCards(cards);

        const column = document.createElement('div');
        column.className = 'kanban-column';
        column.dataset.status = config.status;

        column.innerHTML = `
            <div class="column-header">
                <h3 class="column-title">${config.title}</h3>
                <span class="card-count">${filteredCards.length}</span>
            </div>
            <div class="cards-container" data-status="${config.status}">
                ${filteredCards.length === 0 ? 
                    '<div class="empty-column">No tasks</div>' : 
                    filteredCards.map(card => this.createCardHTML(card)).join('')
                }
            </div>
        `;

        return column;
    }

    createCardHTML(card) {
        const assigneeName = this.getAssigneeName(card.assigned_to);
        const dueDateFormatted = card.due_date ? this.formatDueDate(card.due_date) : '';
        const dueDateClass = this.getDueDateClass(card.due_date);
        
        return `
            <div class="kanban-card priority-${card.priority}" 
                 data-card-id="${card.id}" 
                 draggable="true">
                <div class="card-header">
                    <h4 class="card-title">${this.escapeHtml(card.title)}</h4>
                    <button class="card-menu" onclick="kanban.openCardMenu('${card.id}', event)">⋮</button>
                </div>
                <div class="priority-badge priority-${card.priority}">${card.priority}</div>
                ${card.description ? `<p class="card-description">${this.escapeHtml(card.description)}</p>` : ''}
                <div class="card-meta">
                    <span class="card-assignee">${assigneeName}</span>
                    ${dueDateFormatted ? `<span class="card-due-date ${dueDateClass}">${dueDateFormatted}</span>` : ''}
                </div>
            </div>
        `;
    }

    filterCards(cards) {
        return cards.filter(card => {
            // Search filter
            if (this.searchTerm) {
                const searchLower = this.searchTerm.toLowerCase();
                const matchesSearch = 
                    card.title.toLowerCase().includes(searchLower) ||
                    (card.description && card.description.toLowerCase().includes(searchLower)) ||
                    (card.assigned_to && this.getAssigneeName(card.assigned_to).toLowerCase().includes(searchLower));
                
                if (!matchesSearch) return false;
            }

            // Assignee filter
            if (this.filters.assignee && card.assigned_to !== this.filters.assignee) {
                return false;
            }

            // Priority filter
            if (this.filters.priority && card.priority !== this.filters.priority) {
                return false;
            }

            return true;
        });
    }

    setupColumnDragAndDrop() {
        const containers = document.querySelectorAll('.cards-container');
        const cards = document.querySelectorAll('.kanban-card');

        // Setup card drag events
        cards.forEach(card => {
            card.addEventListener('dragstart', (e) => this.handleDragStart(e));
            card.addEventListener('dragend', (e) => this.handleDragEnd(e));
        });

        // Setup container drop events
        containers.forEach(container => {
            container.addEventListener('dragover', (e) => this.handleDragOver(e));
            container.addEventListener('drop', (e) => this.handleDrop(e));
            container.addEventListener('dragenter', (e) => this.handleDragEnter(e));
            container.addEventListener('dragleave', (e) => this.handleDragLeave(e));
        });
    }

    handleDragStart(e) {
        e.dataTransfer.setData('text/plain', e.target.dataset.cardId);
        e.target.classList.add('dragging');
    }

    handleDragEnd(e) {
        e.target.classList.remove('dragging');
    }

    handleDragOver(e) {
        e.preventDefault();
    }

    handleDragEnter(e) {
        e.preventDefault();
        if (e.target.classList.contains('cards-container')) {
            e.target.classList.add('drag-over');
        }
    }

    handleDragLeave(e) {
        if (e.target.classList.contains('cards-container')) {
            e.target.classList.remove('drag-over');
        }
    }

    async handleDrop(e) {
        e.preventDefault();
        const container = e.target.closest('.cards-container');
        if (!container) return;

        container.classList.remove('drag-over');
        
        const cardId = e.dataTransfer.getData('text/plain');
        const newStatus = container.dataset.status;
        
        await this.moveCard(cardId, newStatus);
    }

    async moveCard(cardId, newStatus) {
        try {
            const response = await fetch(`/api/kanban/${cardId}/move`, {
                method: 'PUT',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    card_id: cardId,
                    new_status: newStatus
                })
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            // Reload board to reflect changes
            await this.loadBoard();
            this.showToast('Task moved successfully', 'success');
        } catch (error) {
            console.error('Error moving card:', error);
            this.showToast('Failed to move task', 'error');
        }
    }

    openTaskModal(card = null) {
        const modal = document.getElementById('task-modal');
        const modalTitle = document.getElementById('modal-title');
        const form = document.getElementById('task-form');

        this.currentEditingCard = card;

        if (card) {
            modalTitle.textContent = 'Edit Task';
            this.populateForm(card);
        } else {
            modalTitle.textContent = 'Add New Task';
            form.reset();
        }

        modal.classList.remove('hidden');
        document.getElementById('task-title').focus();
    }

    closeTaskModal() {
        const modal = document.getElementById('task-modal');
        modal.classList.add('hidden');
        this.currentEditingCard = null;
    }

    populateForm(card) {
        document.getElementById('task-title').value = card.title;
        document.getElementById('task-description').value = card.description || '';
        document.getElementById('task-priority').value = card.priority;
        document.getElementById('task-assignee').value = card.assigned_to || '';
        
        if (card.due_date) {
            const date = new Date(card.due_date);
            document.getElementById('task-due-date').value = date.toISOString().slice(0, 16);
        }
    }

    async handleTaskSubmit(e) {
        e.preventDefault();
        
        const formData = new FormData(e.target);
        const taskData = {
            title: formData.get('title'),
            description: formData.get('description') || null,
            priority: formData.get('priority'),
            assigned_to: formData.get('assigned_to') || null,
            due_date: formData.get('due_date') ? new Date(formData.get('due_date')).toISOString() : null
        };

        try {
            let response;
            if (this.currentEditingCard) {
                // Update existing card
                response = await fetch(`/api/kanban/${this.currentEditingCard.id}`, {
                    method: 'PUT',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify(taskData)
                });
            } else {
                // Create new card
                response = await fetch(`/api/projects/${this.projectId}/kanban`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify(taskData)
                });
            }

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            this.closeTaskModal();
            await this.loadBoard();
            this.showToast(
                this.currentEditingCard ? 'Task updated successfully' : 'Task created successfully', 
                'success'
            );
        } catch (error) {
            console.error('Error saving task:', error);
            this.showToast('Failed to save task', 'error');
        }
    }

    async openCardMenu(cardId, event) {
        event.stopPropagation();
        
        // Find the card data
        const card = this.findCardById(cardId);
        if (!card) return;

        // Simple context menu - in a real app, you might want a proper context menu
        const action = confirm('Edit this task?');
        if (action) {
            this.openTaskModal(card);
        }
    }

    findCardById(cardId) {
        for (const status in this.board.columns) {
            const card = this.board.columns[status].find(c => c.id === cardId);
            if (card) return card;
        }
        return null;
    }

    handleSearch(searchTerm) {
        this.searchTerm = searchTerm;
        this.renderBoard();
    }

    handleFilter(filterType, value) {
        this.filters[filterType] = value;
        this.renderBoard();
    }

    setupRealTimeUpdates() {
        // Setup Server-Sent Events for real-time updates
        if (this.eventSource) {
            this.eventSource.close();
        }

        try {
            this.eventSource = new EventSource(`/api/projects/${this.projectId}/events`);
            
            this.eventSource.onmessage = (event) => {
                const data = JSON.parse(event.data);
                this.handleRealTimeUpdate(data);
            };

            this.eventSource.onerror = (error) => {
                console.error('EventSource failed:', error);
                // Attempt to reconnect after 5 seconds
                setTimeout(() => {
                    if (this.eventSource.readyState === EventSource.CLOSED) {
                        this.setupRealTimeUpdates();
                    }
                }, 5000);
            };
        } catch (error) {
            console.error('Failed to setup real-time updates:', error);
            // Fallback to periodic polling
            this.setupPolling();
        }
    }

    setupPolling() {
        // Fallback polling every 30 seconds
        setInterval(() => {
            this.loadBoard();
        }, 30000);
    }

    handleRealTimeUpdate(data) {
        switch (data.type) {
            case 'card_created':
            case 'card_updated':
            case 'card_moved':
            case 'card_deleted':
                this.loadBoard();
                break;
            default:
                console.log('Unknown update type:', data.type);
        }
    }

    // Utility methods
    getAssigneeName(userId) {
        if (!userId) return 'Unassigned';
        const member = this.teamMembers.find(m => m.user_id === userId);
        return member ? member.name : 'Unknown User';
    }

    formatDueDate(dateString) {
        const date = new Date(dateString);
        const now = new Date();
        const diffTime = date - now;
        const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));

        if (diffDays < 0) {
            return `${Math.abs(diffDays)} days overdue`;
        } else if (diffDays === 0) {
            return 'Due today';
        } else if (diffDays === 1) {
            return 'Due tomorrow';
        } else if (diffDays <= 7) {
            return `Due in ${diffDays} days`;
        } else {
            return date.toLocaleDateString();
        }
    }

    getDueDateClass(dateString) {
        if (!dateString) return '';
        
        const date = new Date(dateString);
        const now = new Date();
        const diffTime = date - now;
        const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));

        if (diffDays < 0) {
            return 'overdue';
        } else if (diffDays <= 2) {
            return 'due-soon';
        }
        return '';
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    showLoading(show) {
        const loading = document.getElementById('loading');
        if (show) {
            loading.classList.remove('hidden');
        } else {
            loading.classList.add('hidden');
        }
    }

    showToast(message, type = 'info') {
        const container = document.getElementById('toast-container');
        const toast = document.createElement('div');
        toast.className = `toast ${type}`;
        toast.textContent = message;

        container.appendChild(toast);

        // Auto remove after 5 seconds
        setTimeout(() => {
            if (toast.parentNode) {
                toast.parentNode.removeChild(toast);
            }
        }, 5000);
    }

    // Cleanup method
    destroy() {
        if (this.eventSource) {
            this.eventSource.close();
        }
    }
}

// Initialize the Kanban board when the page loads
let kanban;
document.addEventListener('DOMContentLoaded', () => {
    kanban = new KanbanBoard();
});

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    if (kanban) {
        kanban.destroy();
    }
});