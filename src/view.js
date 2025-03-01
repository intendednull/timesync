document.addEventListener('DOMContentLoaded', () => {
    // Get schedule ID from URL
    const scheduleId = window.location.pathname.replace('/', '');
    
    // State for schedule and time grid
    let scheduleData = null;
    let isPasswordProtected = false;
    let isEditable = false;
    let currentTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone; // Default to local timezone
    let currentWeekStart = getStartOfWeek(new Date());
    
    // Initialize timezone selector
    populateTimezoneSelector();
    
    // Initialize
    loadSchedule();
    updateDateRange();
    
    // Event listeners for week navigation
    document.getElementById('prev-week').addEventListener('click', () => {
        currentWeekStart.setDate(currentWeekStart.getDate() - 7);
        renderTimeGrid();
        updateDateRange();
    });
    
    document.getElementById('next-week').addEventListener('click', () => {
        currentWeekStart.setDate(currentWeekStart.getDate() + 7);
        renderTimeGrid();
        updateDateRange();
    });
    
    // Share button
    document.getElementById('share-schedule').addEventListener('click', () => {
        const shareLink = document.getElementById('share-link');
        shareLink.value = window.location.href;
        document.getElementById('share-modal').style.display = 'flex';
    });
    
    // Copy link button
    document.getElementById('copy-link').addEventListener('click', () => {
        const shareLink = document.getElementById('share-link');
        shareLink.select();
        document.execCommand('copy');
        document.getElementById('copy-link').textContent = 'Copied!';
        setTimeout(() => {
            document.getElementById('copy-link').textContent = 'Copy Link';
        }, 2000);
    });
    
    // Close share modal
    document.getElementById('close-share').addEventListener('click', () => {
        document.getElementById('share-modal').style.display = 'none';
    });
    
    // Edit schedule button
    document.getElementById('edit-schedule').addEventListener('click', () => {
        if (isPasswordProtected && !isEditable) {
            document.getElementById('password-modal').style.display = 'flex';
        } else {
            window.location.href = `/${scheduleId}/edit`;
        }
    });
    
    // Cancel password button
    document.getElementById('cancel-password').addEventListener('click', () => {
        document.getElementById('password-modal').style.display = 'none';
    });
    
    // Verify password button
    document.getElementById('verify-password').addEventListener('click', async () => {
        const password = document.getElementById('password-input').value;
        
        try {
            const response = await fetch(`/api/schedules/${scheduleId}/verify`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ password })
            });
            
            const data = await response.json();
            
            if (data.valid) {
                // Store in session storage that the password is verified
                sessionStorage.setItem(`schedule_${scheduleId}_verified`, 'true');
                window.location.href = `/${scheduleId}/edit`;
            } else {
                alert('Invalid password. Please try again.');
            }
        } catch (error) {
            console.error('Error verifying password:', error);
            alert('Error verifying password. Please try again.');
        }
    });
    
    // Timezone selector change handler
    document.getElementById('timezone-select').addEventListener('change', (event) => {
        currentTimezone = event.target.value;
        renderTimeGrid();
        updateDateRange();
    });
    
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
    async function loadSchedule() {
        try {
            const response = await fetch(`/api/schedules/${scheduleId}`);
            
            if (!response.ok) {
                throw new Error('Failed to load schedule');
            }
            
            scheduleData = await response.json();
            
            // Update UI with schedule data
            document.getElementById('schedule-name').textContent = scheduleData.name;
            document.getElementById('created-at').textContent = new Date(scheduleData.created_at).toLocaleString();
            
            // Check if schedule is password protected
            isPasswordProtected = !scheduleData.is_editable;
            if (isPasswordProtected) {
                document.getElementById('password-info').style.display = 'block';
                // Check if we have verified the password in this session
                isEditable = sessionStorage.getItem(`schedule_${scheduleId}_verified`) === 'true';
            } else {
                isEditable = true;
            }
            
            // Show edit button if editable or password protected
            if (isEditable || isPasswordProtected) {
                document.getElementById('edit-schedule').style.display = 'block';
            }
            
            // Render the time grid with the schedule data
            renderTimeGrid();
            
        } catch (error) {
            console.error('Error loading schedule:', error);
            document.getElementById('schedule-name').textContent = 'Error loading schedule';
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
    
    function renderTimeGrid() {
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
        
        // Create time rows (all 24 hours in 30-minute increments)
        for (let hour = 0; hour < 24; hour++) {
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
                    
                    // Check if this time slot is in the schedule
                    if (scheduleData && scheduleData.slots) {
                        const isAvailable = scheduleData.slots.some(slot => {
                            const slotStart = new Date(slot.start);
                            const slotEnd = new Date(slot.end);
                            
                            if (slot.is_recurring) {
                                // For recurring slots, just compare day of week and time
                                const isSameDay = slotStart.getDay() === cellDate.getDay();
                                const isSameTime = 
                                    slotStart.getHours() === cellDate.getHours() && 
                                    slotStart.getMinutes() === cellDate.getMinutes();
                                
                                return isSameDay && isSameTime;
                            } else {
                                // For specific dates, check if the date falls within the slot
                                return cellDate >= slotStart && cellDate < slotEnd;
                            }
                        });
                        
                        if (isAvailable) {
                            cell.classList.add('selected');
                            
                            // Add a recurring indicator if applicable
                            const isRecurring = scheduleData.slots.some(slot => 
                                slot.is_recurring && 
                                new Date(slot.start).getDay() === cellDate.getDay() &&
                                new Date(slot.start).getHours() === cellDate.getHours() && 
                                new Date(slot.start).getMinutes() === cellDate.getMinutes()
                            );
                            
                            if (isRecurring) {
                                cell.classList.add('recurring');
                            }
                        }
                    }
                    
                    timeGrid.appendChild(cell);
                }
            }
        }
    }
    
    function formatTime(hour, minute) {
        const period = hour >= 12 ? 'PM' : 'AM';
        const displayHour = hour % 12 || 12;
        return `${displayHour}:${minute.toString().padStart(2, '0')} ${period}`;
    }
});