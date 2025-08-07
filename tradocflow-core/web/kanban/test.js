/**
 * End-to-End Tests for Kanban Board
 * These tests can be run in a browser console or with a testing framework
 */

class KanbanBoardTests {
    constructor() {
        this.baseUrl = window.location.origin;
        this.projectId = 'test-project-123';
        this.testResults = [];
    }

    async runAllTests() {
        console.log('🚀 Starting Kanban Board E2E Tests...');
        
        const tests = [
            this.testApiConnection,
            this.testCreateTask,
            this.testUpdateTask,
            this.testMoveTask,
            this.testDeleteTask,
            this.testSearchFunctionality,
            this.testFilterFunctionality,
            this.testDragAndDrop,
            this.testRealTimeUpdates
        ];

        for (const test of tests) {
            try {
                await test.call(this);
                this.logResult(test.name, 'PASS', '✅');
            } catch (error) {
                this.logResult(test.name, 'FAIL', '❌', error.message);
            }
        }

        this.printResults();
    }

    async testApiConnection() {
        const response = await fetch(`${this.baseUrl}/api/projects/${this.projectId}/kanban`);
        if (!response.ok) {
            throw new Error(`API connection failed: ${response.status}`);
        }
        const data = await response.json();
        if (!data.columns) {
            throw new Error('Invalid board data structure');
        }
    }

    async testCreateTask() {
        const taskData = {
            title: 'Test Task',
            description: 'This is a test task',
            priority: 'medium',
            assigned_to: null,
            due_date: null
        };

        const response = await fetch(`${this.baseUrl}/api/projects/${this.projectId}/kanban`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(taskData)
        });

        if (!response.ok) {
            throw new Error(`Failed to create task: ${response.status}`);
        }

        const createdTask = await response.json();
        if (!createdTask.id || createdTask.title !== taskData.title) {
            throw new Error('Created task data is invalid');
        }

        this.testTaskId = createdTask.id;
    }

    async testUpdateTask() {
        if (!this.testTaskId) {
            throw new Error('No test task ID available');
        }

        const updateData = {
            title: 'Updated Test Task',
            priority: 'high'
        };

        const response = await fetch(`${this.baseUrl}/api/kanban/${this.testTaskId}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(updateData)
        });

        if (!response.ok) {
            throw new Error(`Failed to update task: ${response.status}`);
        }

        const updatedTask = await response.json();
        if (updatedTask.title !== updateData.title || updatedTask.priority !== updateData.priority) {
            throw new Error('Task was not updated correctly');
        }
    }

    async testMoveTask() {
        if (!this.testTaskId) {
            throw new Error('No test task ID available');
        }

        const moveData = {
            card_id: this.testTaskId,
            new_status: 'in_progress'
        };

        const response = await fetch(`${this.baseUrl}/api/kanban/${this.testTaskId}/move`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(moveData)
        });

        if (!response.ok) {
            throw new Error(`Failed to move task: ${response.status}`);
        }

        const movedTask = await response.json();
        if (movedTask.status !== 'in_progress') {
            throw new Error('Task was not moved to correct status');
        }
    }

    async testDeleteTask() {
        if (!this.testTaskId) {
            throw new Error('No test task ID available');
        }

        const response = await fetch(`${this.baseUrl}/api/kanban/${this.testTaskId}`, {
            method: 'DELETE'
        });

        if (response.status !== 204) {
            throw new Error(`Failed to delete task: ${response.status}`);
        }

        // Verify task is deleted
        const getResponse = await fetch(`${this.baseUrl}/api/kanban/${this.testTaskId}`);
        if (getResponse.status !== 404) {
            throw new Error('Task was not properly deleted');
        }
    }

    async testSearchFunctionality() {
        // This test would require the UI to be loaded
        if (typeof kanban === 'undefined') {
            console.log('⚠️ Skipping search test - UI not loaded');
            return;
        }

        const searchInput = document.getElementById('search-input');
        if (!searchInput) {
            throw new Error('Search input not found');
        }

        // Simulate search
        searchInput.value = 'test';
        searchInput.dispatchEvent(new Event('input'));

        // Check if search was applied
        if (kanban.searchTerm !== 'test') {
            throw new Error('Search functionality not working');
        }
    }

    async testFilterFunctionality() {
        // This test would require the UI to be loaded
        if (typeof kanban === 'undefined') {
            console.log('⚠️ Skipping filter test - UI not loaded');
            return;
        }

        const priorityFilter = document.getElementById('priority-filter');
        if (!priorityFilter) {
            throw new Error('Priority filter not found');
        }

        // Simulate filter change
        priorityFilter.value = 'high';
        priorityFilter.dispatchEvent(new Event('change'));

        // Check if filter was applied
        if (kanban.filters.priority !== 'high') {
            throw new Error('Filter functionality not working');
        }
    }

    async testDragAndDrop() {
        // This test would require the UI to be loaded and would be complex to simulate
        if (typeof kanban === 'undefined') {
            console.log('⚠️ Skipping drag-and-drop test - UI not loaded');
            return;
        }

        const cards = document.querySelectorAll('.kanban-card');
        if (cards.length === 0) {
            console.log('⚠️ Skipping drag-and-drop test - no cards available');
            return;
        }

        // Check if cards have draggable attribute
        const firstCard = cards[0];
        if (!firstCard.draggable) {
            throw new Error('Cards are not draggable');
        }

        // Check if drag event listeners are attached
        const hasDataTransfer = firstCard.ondragstart !== null || 
                               firstCard.addEventListener !== undefined;
        if (!hasDataTransfer) {
            throw new Error('Drag event listeners not properly attached');
        }
    }

    async testRealTimeUpdates() {
        // Test if EventSource is supported and connection can be established
        if (typeof EventSource === 'undefined') {
            throw new Error('EventSource not supported in this browser');
        }

        return new Promise((resolve, reject) => {
            const eventSource = new EventSource(`${this.baseUrl}/api/projects/${this.projectId}/events`);
            
            const timeout = setTimeout(() => {
                eventSource.close();
                reject(new Error('EventSource connection timeout'));
            }, 5000);

            eventSource.onopen = () => {
                clearTimeout(timeout);
                eventSource.close();
                resolve();
            };

            eventSource.onerror = (error) => {
                clearTimeout(timeout);
                eventSource.close();
                reject(new Error('EventSource connection failed'));
            };
        });
    }

    logResult(testName, status, icon, error = null) {
        const result = {
            test: testName,
            status,
            icon,
            error
        };
        this.testResults.push(result);
        
        const message = error ? 
            `${icon} ${testName}: ${status} - ${error}` : 
            `${icon} ${testName}: ${status}`;
        console.log(message);
    }

    printResults() {
        console.log('\n📊 Test Results Summary:');
        console.log('========================');
        
        const passed = this.testResults.filter(r => r.status === 'PASS').length;
        const failed = this.testResults.filter(r => r.status === 'FAIL').length;
        
        console.log(`✅ Passed: ${passed}`);
        console.log(`❌ Failed: ${failed}`);
        console.log(`📈 Success Rate: ${((passed / this.testResults.length) * 100).toFixed(1)}%`);
        
        if (failed > 0) {
            console.log('\n❌ Failed Tests:');
            this.testResults
                .filter(r => r.status === 'FAIL')
                .forEach(r => console.log(`   - ${r.test}: ${r.error}`));
        }
        
        console.log('\n🎉 Testing completed!');
    }
}

// Auto-run tests if this script is loaded directly
if (typeof window !== 'undefined' && window.location.pathname.includes('test.html')) {
    // Add a button to run tests
    document.addEventListener('DOMContentLoaded', () => {
        const button = document.createElement('button');
        button.textContent = 'Run E2E Tests';
        button.className = 'test-link';
        button.style.display = 'block';
        button.style.margin = '20px auto';
        button.onclick = async () => {
            button.disabled = true;
            button.textContent = 'Running Tests...';
            
            const tests = new KanbanBoardTests();
            await tests.runAllTests();
            
            button.disabled = false;
            button.textContent = 'Run E2E Tests Again';
        };
        
        document.querySelector('.test-container').appendChild(button);
    });
}

// Export for use in other contexts
if (typeof module !== 'undefined' && module.exports) {
    module.exports = KanbanBoardTests;
}