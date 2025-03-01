document.addEventListener('DOMContentLoaded', () => {
    // State for schedules and availability data
    const schedules = [];
    let currentWeekStart = getStartOfWeek(new Date());
    let tooltip = null;
    
    // Check for schedule IDs in URL query params
    const urlParams = new URLSearchParams(window.location.search);
    const scheduleIds = urlParams.get('ids')?.split(',') || [];
    
    // Create tooltip element
    createTooltip();
    
    // Initialize
    initializeFromUrl();
    updateDateRange();
    
    // Add schedule button
    document.getElementById('add-schedule-btn').addEventListener('click', async () => {
        const scheduleIdInput = document.getElementById('schedule-id-input');
        const scheduleId = scheduleIdInput.value.trim();
        
        if (!scheduleId) {
            alert('Please enter a schedule ID');
            return;
        }
        
        try {
            await addSchedule(scheduleId);
            scheduleIdInput.value = ''; // Clear input
        } catch (error) {
            alert(`Error: ${error.message}`);
        }
    });
    
    // Event listeners for week navigation
    document.getElementById('prev-week').addEventListener('click', () => {
        currentWeekStart.setDate(currentWeekStart.getDate() - 7);
        renderComparisonGrid();
        updateDateRange();
    });
    
    document.getElementById('next-week').addEventListener('click', () => {
        currentWeekStart.setDate(currentWeekStart.getDate() + 7);
        renderComparisonGrid();
        updateDateRange();
    });
    
    // Functions
    async function initializeFromUrl() {
        if (scheduleIds.length > 0) {
            try {
                for (const id of scheduleIds) {
                    await addSchedule(id);
                }
            } catch (error) {
                console.error('Error loading schedules from URL:', error);
            }
        }
    }
    
    function createTooltip() {
        tooltip = document.createElement('div');
        tooltip.classList.add('grid-cell-tooltip');
        document.body.appendChild(tooltip);
    }
    
    async function addSchedule(scheduleId) {
        // Check if schedule is already added
        if (schedules.some(s => s.id === scheduleId)) {
            alert('This schedule has already been added');
            return;
        }
        
        try {
            const response = await fetch(`/api/schedules/${scheduleId}`);
            
            if (!response.ok) {
                throw new Error('Schedule not found');
            }
            
            const scheduleData = await response.json();
            schedules.push(scheduleData);
            
            // Update UI
            renderScheduleList();
            updateUrl();
            
            // Only show comparison if we have schedules
            if (schedules.length > 0) {
                document.getElementById('comparison-view').style.display = 'block';
                renderComparisonGrid();
            }
            
        } catch (error) {
            console.error('Error loading schedule:', error);
            throw new Error('Could not load schedule. Please check the ID and try again.');
        }
    }
    
    function removeSchedule(scheduleId) {
        const index = schedules.findIndex(s => s.id === scheduleId);
        if (index !== -1) {
            schedules.splice(index, 1);
            renderScheduleList();
            updateUrl();
            
            if (schedules.length === 0) {
                document.getElementById('comparison-view').style.display = 'none';
            } else {
                renderComparisonGrid();
            }
        }
    }
    
    function updateUrl() {
        // Update URL with current schedule IDs without reloading page
        const ids = schedules.map(s => s.id).join(',');
        const newUrl = ids.length > 0 ? `/availability?ids=${ids}` : '/availability';
        window.history.replaceState({}, '', newUrl);
    }
    
    function renderScheduleList() {
        const schedulesListElement = document.getElementById('schedules-list');
        const emptyState = document.getElementById('empty-schedules');
        
        // Remove all schedule items first
        const existingItems = document.querySelectorAll('.schedule-item');
        existingItems.forEach(item => item.remove());
        
        // Show empty state if no schedules
        if (schedules.length === 0) {
            emptyState.style.display = 'flex';
            return;
        }
        
        // Hide empty state and add schedules
        emptyState.style.display = 'none';
        
        schedules.forEach(schedule => {
            const scheduleItem = document.createElement('div');
            scheduleItem.classList.add('schedule-item');
            
            scheduleItem.innerHTML = `
                <div class="schedule-details">
                    <div class="schedule-name">${schedule.name}</div>
                    <div class="schedule-id">ID: ${schedule.id}</div>
                </div>
                <button class="remove-schedule" data-id="${schedule.id}">×</button>
            `;
            
            // Add remove button handler
            scheduleItem.querySelector('.remove-schedule').addEventListener('click', (e) => {
                const id = e.target.getAttribute('data-id');
                removeSchedule(id);
            });
            
            schedulesListElement.appendChild(scheduleItem);
        });
    }
    
    function getStartOfWeek(date) {
        const result = new Date(date);
        const day = result.getDay();
        const diff = result.getDate() - day + (day === 0 ? -6 : 1); // Adjust for Sunday
        result.setDate(diff);
        result.setHours(0, 0, 0, 0);
        return result;
    }
    
    function formatDate(date) {
        return date.toLocaleDateString('en-US', { 
            month: 'short', 
            day: 'numeric',
            year: date.getFullYear() !== new Date().getFullYear() ? 'numeric' : undefined
        });
    }
    
    function updateDateRange() {
        const weekEnd = new Date(currentWeekStart);
        weekEnd.setDate(weekEnd.getDate() + 6);
        
        document.getElementById('current-date-range').textContent = 
            `${formatDate(currentWeekStart)} - ${formatDate(weekEnd)}`;
    }
    
    function renderComparisonGrid() {
        const timeGrid = document.getElementById('time-grid');
        timeGrid.innerHTML = '';
        
        // Create header row with days
        const headerRow = document.createElement('div');
        headerRow.classList.add('grid-header');
        headerRow.style.gridColumn = '1';
        timeGrid.appendChild(headerRow);
        
        for (let i = 0; i < 7; i++) {
            const day = new Date(currentWeekStart);
            day.setDate(day.getDate() + i);
            
            const dayHeader = document.createElement('div');
            dayHeader.classList.add('grid-header');
            dayHeader.textContent = day.toLocaleDateString('en-US', { weekday: 'short' }) + 
                                   ' ' + day.getDate();
            timeGrid.appendChild(dayHeader);
        }
        
        // Create availability map for each 30-min slot
        const availabilityMap = createAvailabilityMap();
        
        // Create time rows (from 8am to 10pm in 30-minute increments)
        for (let hour = 8; hour < 22; hour++) {
            for (let minute = 0; minute < 60; minute += 30) {
                const timeLabel = document.createElement('div');
                timeLabel.classList.add('time-label');
                timeLabel.textContent = formatTime(hour, minute);
                timeGrid.appendChild(timeLabel);
                
                // Create cells for each day
                for (let day = 0; day < 7; day++) {
                    const cell = document.createElement('div');
                    cell.classList.add('grid-cell');
                    
                    // Store date and time info as data attributes
                    const cellDate = new Date(currentWeekStart);
                    cellDate.setDate(cellDate.getDate() + day);
                    cellDate.setHours(hour, minute, 0, 0);
                    
                    const timeKey = cellDate.toISOString();
                    const availableCount = availabilityMap[timeKey] || 0;
                    const totalSchedules = schedules.length;
                    
                    // Skip if no schedules
                    if (totalSchedules > 0) {
                        // Calculate percentage of availability
                        const availabilityPercentage = (availableCount / totalSchedules) * 100;
                        const heatLevel = Math.round(availabilityPercentage / 10); // 0-10 scale
                        
                        cell.dataset.heat = heatLevel;
                        cell.dataset.available = availableCount;
                        cell.dataset.total = totalSchedules;
                        
                        // Add tooltip functionality
                        cell.addEventListener('mouseenter', showTooltip);
                        cell.addEventListener('mouseleave', hideTooltip);
                    }
                    
                    timeGrid.appendChild(cell);
                }
            }
        }
    }
    
    function createAvailabilityMap() {
        const availabilityMap = {};
        
        // Process each schedule's time slots
        schedules.forEach(schedule => {
            if (schedule.slots) {
                schedule.slots.forEach(slot => {
                    const slotStart = new Date(slot.start);
                    const slotEnd = new Date(slot.end);
                    
                    // Create 30-minute increments within this slot
                    let current = new Date(slotStart);
                    while (current < slotEnd) {
                        const timeKey = current.toISOString();
                        
                        // Increment count for this time slot
                        availabilityMap[timeKey] = (availabilityMap[timeKey] || 0) + 1;
                        
                        // Move to next 30-minute slot
                        current.setMinutes(current.getMinutes() + 30);
                    }
                });
            }
        });
        
        return availabilityMap;
    }
    
    function formatTime(hour, minute) {
        const period = hour >= 12 ? 'PM' : 'AM';
        const displayHour = hour % 12 || 12;
        return `${displayHour}:${minute.toString().padStart(2, '0')} ${period}`;
    }
    
    function showTooltip(event) {
        const cell = event.target;
        const availableCount = cell.dataset.available;
        const totalCount = cell.dataset.total;
        
        if (!availableCount || !totalCount) return;
        
        // Format tooltip content
        tooltip.textContent = `${availableCount} of ${totalCount} available`;
        
        // Position tooltip near the cell
        const rect = cell.getBoundingClientRect();
        tooltip.style.left = `${rect.left + window.scrollX + rect.width/2 - tooltip.offsetWidth/2}px`;
        tooltip.style.top = `${rect.top + window.scrollY - tooltip.offsetHeight - 5}px`;
        
        tooltip.style.display = 'block';
    }
    
    function hideTooltip() {
        tooltip.style.display = 'none';
    }
});