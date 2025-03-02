document.addEventListener('DOMContentLoaded', () => {
    // State management for selected time slots
    const selectedTimeSlots = new Set();
    let isMouseDown = false;
    let isSelecting = true; // true = selecting, false = deselecting
    let isRecurringMode = true; // Always set to true - weekly recurring is the only supported mode now
    let currentTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone; // Default to local timezone
    
    // Box selection state variables
    let selectionStart = null;
    let selectionEnd = null;
    let previouslySelectedCells = new Set();
    
    // Date handling
    let currentWeekStart = getStartOfWeek(new Date());
    
    // Initialize timezone selector
    populateTimezoneSelector();
    
    // Initialize the time grid
    initializeTimeGrid();
    updateDateRange();
    
    // Now that the DOM is initialized, check for URL parameters and fill form fields
    setTimeout(() => {
        const urlParams = new URLSearchParams(window.location.search);
        const discordId = urlParams.get('discord_id');
        const nameParam = urlParams.get('name');
        
        // Set the name field if it exists in the URL
        if (nameParam) {
            const nameField = document.getElementById('schedule-name');
            if (nameField) {
                nameField.value = decodeURIComponent(nameParam);
                console.log('Name field updated:', nameField.value);
            }
        }
        
        // Set the discord_id field if it exists in the URL
        if (discordId) {
            const discordField = document.getElementById('discord-id');
            if (discordField) {
                discordField.value = discordId;
                console.log('Discord ID field updated:', discordField.value);
            }
        }
    }, 10); // Small delay to ensure DOM is fully ready
    
    // Event listeners for week navigation (preserved for potential future use)
    const prevWeekButton = document.getElementById('prev-week');
    if (prevWeekButton) {
        prevWeekButton.addEventListener('click', () => {
            if (!isRecurringMode) {
                currentWeekStart.setDate(currentWeekStart.getDate() - 7);
                initializeTimeGrid();
                updateDateRange();
            }
        });
    }
    
    const nextWeekButton = document.getElementById('next-week');
    if (nextWeekButton) {
        nextWeekButton.addEventListener('click', () => {
            if (!isRecurringMode) {
                currentWeekStart.setDate(currentWeekStart.getDate() + 7);
                initializeTimeGrid();
                updateDateRange();
            }
        });
    }
    
    // Set up references to UI elements (preserved for potential future use)
    const recurringToggle = document.getElementById('recurring-toggle');
    const scheduleDescription = document.getElementById('schedule-mode-description');
    const dateRangeText = document.getElementById('current-date-range');
    const timeControlsDiv = document.querySelector('.time-controls');
    const weeklyHeaderDiv = document.getElementById('weekly-header');
    
    // Always set the toggle to checked to enforce weekly recurring mode
    if (recurringToggle) {
        recurringToggle.checked = true;
    }
    
    // Always show weekly header since we only support recurring weekly schedules
    if (weeklyHeaderDiv) {
        weeklyHeaderDiv.style.display = 'block';
    }
    
    // The toggle event listener is preserved for potential future use but not active in UI
    if (recurringToggle) {
        recurringToggle.addEventListener('change', () => {
            // Functionality preserved but inactive in the UI
            // Always reset to true as we only support weekly scheduling
            isRecurringMode = true;
            recurringToggle.checked = true;
            
            // Show weekly header
            if (weeklyHeaderDiv) {
                weeklyHeaderDiv.style.display = 'block';
            }
            
            // Hide date controls
            if (timeControlsDiv) {
                timeControlsDiv.style.display = 'none';
            }
            
            // Update description
            if (scheduleDescription) {
                scheduleDescription.textContent = 'Setting a weekly recurring pattern. All selected times will repeat weekly.';
            }
            
            // Reset to current week
            currentWeekStart = getStartOfWeek(new Date());
            
            // Clear selections and reinitialize grid
            selectedTimeSlots.clear();
            initializeTimeGrid();
        });
    }
    
    // Timezone selector change handler
    document.getElementById('timezone-select').addEventListener('change', (event) => {
        currentTimezone = event.target.value;
        initializeTimeGrid();
        updateDateRange();
    });
    
    // Form submission handler
    document.getElementById('schedule-form').addEventListener('submit', handleFormSubmit);
    
    // Functions
    function populateTimezoneSelector() {
        const timezoneSelect = document.getElementById('timezone-select');
        
        // List of common timezones
        const timezones = [
            'Pacific/Honolulu', // -10:00
            'America/Anchorage', // -09:00
            'America/Los_Angeles', // -08:00
            'America/Denver', // -07:00
            'America/Chicago', // -06:00
            'America/New_York', // -05:00
            'America/Halifax', // -04:00
            'America/St_Johns', // -03:30
            'America/Sao_Paulo', // -03:00
            'Atlantic/Cape_Verde', // -01:00
            'Europe/London', // +00:00
            'Europe/Paris', // +01:00
            'Europe/Helsinki', // +02:00
            'Europe/Moscow', // +03:00
            'Asia/Dubai', // +04:00
            'Asia/Karachi', // +05:00
            'Asia/Dhaka', // +06:00
            'Asia/Bangkok', // +07:00
            'Asia/Singapore', // +08:00
            'Asia/Tokyo', // +09:00
            'Australia/Sydney', // +10:00
            'Pacific/Auckland', // +12:00
        ];
        
        // Clear existing options
        timezoneSelect.innerHTML = '';
        
        // Add options for each timezone
        timezones.forEach(tz => {
            const option = document.createElement('option');
            option.value = tz;
            
            // Display timezone with offset
            try {
                const now = new Date();
                const tzName = new Intl.DateTimeFormat('en', { 
                    timeZone: tz, 
                    timeZoneName: 'short' 
                }).formatToParts(now).find(part => part.type === 'timeZoneName').value;
                
                // Calculate offset
                const tzOffset = new Date().toLocaleString('en-US', { timeZone: tz, timeZoneName: 'longOffset' })
                    .split('GMT')[1];
                
                option.text = `${tz.replace('_', ' ')} (${tzName}, GMT${tzOffset})`;
            } catch (e) {
                option.text = tz;
            }
            
            // Select the user's timezone by default
            if (tz === currentTimezone) {
                option.selected = true;
            }
            
            timezoneSelect.appendChild(option);
        });
        
        // Add user's local timezone if not in the list
        if (!timezones.includes(currentTimezone)) {
            const option = document.createElement('option');
            option.value = currentTimezone;
            option.text = `${currentTimezone} (Local)`;
            option.selected = true;
            timezoneSelect.appendChild(option);
        }
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
    
    function initializeTimeGrid() {
        const timeGrid = document.getElementById('time-grid');
        timeGrid.innerHTML = '';
        selectedTimeSlots.clear();
        
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
            
            if (isRecurringMode) {
                // In recurring mode, only show day names without dates
                dayHeader.textContent = day.toLocaleDateString('en-US', { weekday: 'short' });
                dayHeader.classList.add('recurring-header');
            } else {
                // In specific date mode, show day name and date
                dayHeader.textContent = day.toLocaleDateString('en-US', { weekday: 'short' }) + 
                                       ' ' + day.getDate();
            }
            
            timeGrid.appendChild(dayHeader);
        }
        
        // Create time rows (all 24 hours in 1-hour increments)
        for (let hour = 0; hour < 24; hour++) {
            const rowIndex = hour;
            
            const timeLabel = document.createElement('div');
            timeLabel.classList.add('time-label');
            timeLabel.textContent = formatTime(hour, 0);
            timeGrid.appendChild(timeLabel);
            
            // Create cells for each day
            for (let day = 0; day < 7; day++) {
                const cell = document.createElement('div');
                cell.classList.add('grid-cell');
                
                // Store date and time info as data attributes
                const cellDate = new Date(currentWeekStart);
                cellDate.setDate(cellDate.getDate() + day);
                cellDate.setHours(hour, 0, 0, 0);
                
                cell.dataset.time = cellDate.toISOString();
                
                // Store grid coordinates for easy box selection
                cell.dataset.row = rowIndex;
                cell.dataset.col = day;
                
                // Add event listeners for drag selection
                cell.addEventListener('mousedown', handleMouseDown);
                cell.addEventListener('mouseenter', handleMouseEnter);
                cell.addEventListener('mouseup', handleMouseUp);
                
                timeGrid.appendChild(cell);
            }
        }
        
        // Add event listener to handle mouseup outside the grid
        document.addEventListener('mouseup', () => {
            isMouseDown = false;
        });
    }
    
    function formatTime(hour, minute) {
        const period = hour >= 12 ? 'PM' : 'AM';
        const displayHour = hour % 12 || 12;
        return `${displayHour}:${minute.toString().padStart(2, '0')} ${period}`;
    }
    
    function handleMouseDown(event) {
        const cell = event.target;
        
        // Only handle clicks on grid cells
        if (!cell.classList.contains('grid-cell')) return;
        
        isMouseDown = true;
        
        // Save the initially selected cells to restore them if needed
        previouslySelectedCells.clear();
        document.querySelectorAll('.grid-cell.selected').forEach(cell => {
            previouslySelectedCells.add(cell);
        });
        
        // Determine if we're selecting or deselecting based on current cell state
        isSelecting = !cell.classList.contains('selected');
        
        // Store the starting cell position
        selectionStart = {
            row: cell.dataset.row,
            col: cell.dataset.col
        };
        
        // Initialize end to same as start
        selectionEnd = { ...selectionStart };
        
        // Mark the start cell
        toggleCellState(cell, isSelecting);
        
        event.preventDefault(); // Prevent text selection
    }
    
    function handleMouseEnter(event) {
        const cell = event.target;
        
        // Only handle events on grid cells
        if (!isMouseDown || !cell.classList.contains('grid-cell')) return;
        
        // Update the end cell position
        selectionEnd = {
            row: cell.dataset.row,
            col: cell.dataset.col
        };
        
        // Apply the box selection
        applyBoxSelection();
    }
    
    function handleMouseUp() {
        if (!isMouseDown) return;
        
        // Finalize the selection
        applyBoxSelection(true); // true = final selection
        
        // Reset selection state
        isMouseDown = false;
        selectionStart = null;
        selectionEnd = null;
        previouslySelectedCells.clear();
    }
    
    function toggleCell(cell) {
        toggleCellState(cell, isSelecting);
    }
    
    function toggleCellState(cell, selecting) {
        const timeValue = cell.dataset.time;
        
        if (selecting) {
            cell.classList.add('selected');
            selectedTimeSlots.add(timeValue);
        } else {
            cell.classList.remove('selected');
            selectedTimeSlots.delete(timeValue);
        }
    }
    
    function applyBoxSelection(isFinal = false) {
        if (!selectionStart || !selectionEnd) return;
        
        // Calculate the box boundaries
        const minRow = Math.min(parseInt(selectionStart.row), parseInt(selectionEnd.row));
        const maxRow = Math.max(parseInt(selectionStart.row), parseInt(selectionEnd.row));
        const minCol = Math.min(parseInt(selectionStart.col), parseInt(selectionEnd.col));
        const maxCol = Math.max(parseInt(selectionStart.col), parseInt(selectionEnd.col));
        
        // Reset temporary visual selection state
        document.querySelectorAll('.grid-cell.box-selecting').forEach(cell => {
            cell.classList.remove('box-selecting');
        });
        
        // Start with a clean slate - return to initial state before drag
        if (!isFinal) {
            // Reset cell states to match the selectedTimeSlots set
            document.querySelectorAll('.grid-cell').forEach(cell => {
                const isInSelectedSet = selectedTimeSlots.has(cell.dataset.time);
                if (isInSelectedSet) {
                    cell.classList.add('selected');
                } else {
                    cell.classList.remove('selected');
                }
            });
        }
        
        // Apply box selection to all cells in the selection rectangle
        const timeGrid = document.getElementById('time-grid');
        for (let row = minRow; row <= maxRow; row++) {
            for (let col = minCol; col <= maxCol; col++) {
                const cell = timeGrid.querySelector(`.grid-cell[data-row="${row}"][data-col="${col}"]`);
                if (cell) {
                    if (!isFinal) {
                        // During dragging, highlight cells
                        cell.classList.add('box-selecting');
                    }
                    
                    // Toggle the cell state
                    toggleCellState(cell, isSelecting);
                }
            }
        }
    }
    
    async function handleFormSubmit(event) {
        event.preventDefault();
        
        if (selectedTimeSlots.size === 0) {
            alert('Please select at least one time slot.');
            return;
        }
        
        const name = document.getElementById('schedule-name').value;
        const password = ''; // Password functionality is disabled
        const discordId = document.getElementById('discord-id').value;
        const submitButton = document.getElementById('create-schedule');
        
        // Convert selected times to slot format
        const slots = [];
        const sortedTimes = Array.from(selectedTimeSlots).sort();
        const sortedDates = sortedTimes.map(time => new Date(time)).sort((a, b) => a - b);
        console.log('Sorted dates:', sortedDates.map(d => d.toISOString()));
        
        if (isRecurringMode) {
            // Group dates by day of week
            const dayGroups = new Map(); // Map of day of week -> array of dates
            
            sortedDates.forEach(date => {
                const dayOfWeek = date.getDay();
                if (!dayGroups.has(dayOfWeek)) {
                    dayGroups.set(dayOfWeek, []);
                }
                dayGroups.get(dayOfWeek).push(date);
            });
            
            // Process each day group
            for (const [dayOfWeek, dates] of dayGroups.entries()) {
                // Sort dates within each day group by hour
                dates.sort((a, b) => a.getHours() - b.getHours());
                
                let startTime = null;
                let endTime = null;
                
                // Process slots for this day of week
                for (let i = 0; i < dates.length; i++) {
                    const currentDate = dates[i];
                    
                    if (startTime === null) {
                        // Start a new slot
                        startTime = currentDate;
                        endTime = new Date(startTime);
                        endTime.setHours(endTime.getHours() + 1);
                    } else {
                        // Check if this is adjacent to the previous slot
                        const expectedHour = endTime.getHours();
                        if (currentDate.getHours() === expectedHour) {
                            // Extend the current slot
                            endTime.setHours(endTime.getHours() + 1);
                        } else {
                            // Add the completed slot and start a new one
                            slots.push({
                                start: startTime.toISOString(),
                                end: endTime.toISOString(),
                                is_recurring: true
                            });
                            
                            startTime = currentDate;
                            endTime = new Date(startTime);
                            endTime.setHours(endTime.getHours() + 1);
                        }
                    }
                }
                
                // Add the last slot for this day
                if (startTime !== null) {
                    slots.push({
                        start: startTime.toISOString(),
                        end: endTime.toISOString(),
                        is_recurring: true
                    });
                }
            }
        } else {
            // Non-recurring mode - process chronologically
            let startTime = null;
            let endTime = null;
            
            // Merge adjacent time slots with simplified approach
            if (sortedDates.length > 0) {
                startTime = sortedDates[0];
                endTime = new Date(startTime);
                endTime.setHours(endTime.getHours() + 1);
                
                for (let i = 1; i < sortedDates.length; i++) {
                    const currentTime = sortedDates[i];
                    const previousEndTime = new Date(endTime);
                    
                    // Check if this slot is adjacent to the previous one
                    if (currentTime.getTime() === previousEndTime.getTime()) {
                        // Extend the current time slot
                        endTime.setHours(endTime.getHours() + 1);
                    } else {
                        // Add the completed slot
                        slots.push({
                            start: startTime.toISOString(),
                            end: endTime.toISOString(),
                            is_recurring: false
                        });
                        
                        // Start a new slot
                        startTime = currentTime;
                        endTime = new Date(currentTime);
                        endTime.setHours(endTime.getHours() + 1);
                    }
                }
            }
            
            // Add the last slot if there is one
            if (startTime !== null) {
                slots.push({
                    start: startTime.toISOString(),
                    end: endTime.toISOString(),
                    is_recurring: false
                });
            }
        }
        
        // Validate slots before sending
        if (slots.length === 0) {
            alert('No valid time slots were generated. Please try again.');
            return;
        }
        
        // Validate each slot to ensure start is before end
        for (const slot of slots) {
            const start = new Date(slot.start);
            const end = new Date(slot.end);
            if (start >= end) {
                console.error('Invalid slot detected:', slot);
                alert('Error: Invalid time slot detected. The start time must be before the end time.');
                return;
            }
        }
        
        // Log the slots for debugging
        console.log('Final merged slots:', slots);
        
        // Prepare request data
        const requestData = {
            name,
            slots,
            timezone: currentTimezone // Include the timezone
        };
        
        // Add optional fields if provided
        if (password) requestData.password = password;
        if (discordId) requestData.discord_id = discordId;
        
        // Store the original button text before changing it
        const originalText = submitButton.textContent;
        
        try {
            // Show loading state
            submitButton.textContent = 'Creating...';
            submitButton.disabled = true;
            
            // Debug: log request data
            console.log('Sending schedule data:', JSON.stringify(requestData, null, 2));
            
            // Send the request to the API
            const response = await fetch('/api/schedules', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(requestData)
            });
            
            // Debug: log response status
            console.log('Response status:', response.status);
            
            if (!response.ok) {
                const errorText = await response.text();
                console.log('Error response:', errorText);
                
                let errorMessage = 'Failed to create schedule';
                try {
                    const errorData = JSON.parse(errorText);
                    errorMessage = errorData.message || errorMessage;
                    console.log('Parsed error data:', errorData);
                } catch (e) {
                    // If the response isn't JSON, use the text as the error message
                    errorMessage = errorText || errorMessage;
                    console.log('Error parsing error response:', e);
                }
                throw new Error(errorMessage);
            }
            
            const data = await response.json();
            
            // Redirect to the new schedule page
            window.location.href = `/${data.id}`;
        } catch (error) {
            console.error('Error creating schedule:', error);
            alert(`Error: ${error.message}`);
            
            // Reset button state
            submitButton.textContent = originalText;
            submitButton.disabled = false;
        }
    }
});