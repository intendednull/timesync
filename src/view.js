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
    
    // Event listeners for week navigation (preserved for potential future use)
    const prevWeekButton = document.getElementById('prev-week');
    if (prevWeekButton) {
        prevWeekButton.addEventListener('click', () => {
            currentWeekStart.setDate(currentWeekStart.getDate() - 7);
            renderTimeGrid();
            updateDateRange();
        });
    }
    
    const nextWeekButton = document.getElementById('next-week');
    if (nextWeekButton) {
        nextWeekButton.addEventListener('click', () => {
            currentWeekStart.setDate(currentWeekStart.getDate() + 7);
            renderTimeGrid();
            updateDateRange();
        });
    }
    
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
    
    // Edit schedule button (preserved for potential future use)
    const editButton = document.getElementById('edit-schedule');
    if (editButton) {
        editButton.addEventListener('click', () => {
            // Editing functionality is disabled, but code is preserved
            if (isPasswordProtected && !isEditable) {
                document.getElementById('password-modal').style.display = 'flex';
            } else {
                window.location.href = `/${scheduleId}/edit`;
            }
        });
    }
    
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
        
        // Debug log current timezone
        console.log('Changed timezone to:', currentTimezone);
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
            
            // Debug log the slots
            console.log('Loaded schedule data:', scheduleData);
            if (scheduleData.slots) {
                scheduleData.slots.forEach((slot, index) => {
                    console.log(`Slot ${index}:`, {
                        start: new Date(slot.start).toLocaleString(),
                        end: new Date(slot.end).toLocaleString(),
                        is_recurring: slot.is_recurring
                    });
                });
            }
            
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
            
            // Edit functionality disabled - button always remains hidden
            // Code preserved for potential future use
            /*
            if (isEditable || isPasswordProtected) {
                document.getElementById('edit-schedule').style.display = 'block';
            }
            */
            
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
            dayHeader.classList.add('grid-header', 'recurring-header');
            // Show only day names without dates for weekly recurring schedule
            dayHeader.textContent = day.toLocaleDateString('en-US', { weekday: 'short' });
            timeGrid.appendChild(dayHeader);
        }
        
        // Create time rows (all 24 hours in 1-hour increments)
        for (let hour = 0; hour < 24; hour++) {
            const minute = 0; // Only use whole hours
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
                        // Convert slot times from UTC to the selected timezone
                        const slotStart = convertToTimezone(new Date(slot.start), currentTimezone);
                        const slotEnd = convertToTimezone(new Date(slot.end), currentTimezone);
                        
                        if (slot.is_recurring) {
                            // For recurring slots, we need to match the day of week and hour
                            const cellDay = cellDate.getDay();
                            const slotDay = slotStart.getDay();
                            const isSameDay = cellDay === slotDay;
                            
                            if (!isSameDay) return false;
                            
                            // Get hours only (ignoring minutes for 1-hour increments)
                            const cellHour = cellDate.getHours();
                            const slotStartHour = slotStart.getHours();
                            const slotEndHour = slotEnd.getHours();
                            
                            // Handle day wrapping (if end time is on next day)
                            if (slotStartHour >= slotEndHour && slotEndHour > 0) {
                                // If slot crosses midnight (e.g., 10 PM - 2 AM)
                                return cellHour >= slotStartHour || cellHour < slotEndHour;
                            } else {
                                // Normal case (e.g., 9 AM - 5 PM)
                                return cellHour >= slotStartHour && cellHour < slotEndHour;
                            }
                        } else {
                            // For specific dates, we need to compare dates accounting for timezone
                            
                            // Create datetime objects with current timezone
                            const cellDateTime = new Date(
                                cellDate.getFullYear(),
                                cellDate.getMonth(),
                                cellDate.getDate(),
                                cellDate.getHours()
                            );
                            
                            // Get the slot start/end in current timezone
                            const slotStartDateTime = new Date(
                                slotStart.getFullYear(),
                                slotStart.getMonth(),
                                slotStart.getDate(),
                                slotStart.getHours()
                            );
                            
                            const slotEndDateTime = new Date(
                                slotEnd.getFullYear(),
                                slotEnd.getMonth(),
                                slotEnd.getDate(),
                                slotEnd.getHours()
                            );
                            
                            // Compare date-hour in current timezone
                            return cellDateTime >= slotStartDateTime && cellDateTime < slotEndDateTime;
                        }
                    });
                    
                    if (isAvailable) {
                        cell.classList.add('selected');
                        
                        // Add a recurring indicator if applicable
                        const isRecurring = scheduleData.slots.some(slot => {
                            if (!slot.is_recurring) return false;
                            
                            // Convert slot times from UTC to the selected timezone
                            const slotStart = convertToTimezone(new Date(slot.start), currentTimezone);
                            const slotEnd = convertToTimezone(new Date(slot.end), currentTimezone);
                            
                            // Check day of week
                            const cellDay = cellDate.getDay();
                            const slotDay = slotStart.getDay();
                            if (cellDay !== slotDay) return false;
                            
                            // Check hour only (for 1-hour increments)
                            const cellHour = cellDate.getHours();
                            const slotStartHour = slotStart.getHours();
                            const slotEndHour = slotEnd.getHours();
                            
                            // Handle day wrapping (if end time is on next day)
                            if (slotStartHour >= slotEndHour && slotEndHour > 0) {
                                // If slot crosses midnight (e.g., 10 PM - 2 AM)
                                return cellHour >= slotStartHour || cellHour < slotEndHour;
                            } else {
                                // Normal case (e.g., 9 AM - 5 PM)
                                return cellHour >= slotStartHour && cellHour < slotEndHour;
                            }
                        });
                        
                        if (isRecurring) {
                            cell.classList.add('recurring');
                        }
                    }
                }
                
                timeGrid.appendChild(cell);
            }
        }
    }
    
    function formatTime(hour, minute) {
        const period = hour >= 12 ? 'PM' : 'AM';
        const displayHour = hour % 12 || 12;
        return `${displayHour}:${minute.toString().padStart(2, '0')} ${period}`;
    }
    
    /**
     * Converts a date to the specified timezone
     * @param {Date} date - The date to convert
     * @param {string} timezone - The timezone to convert to
     * @returns {Date} - A new Date object in the specified timezone
     */
    function convertToTimezone(date, timezone) {
        // Get ISO string in the desired timezone
        const options = { timeZone: timezone };
        
        // If invalid timezone, fallback to local
        try {
            // Create a formatter for each part of the date
            const yearFormatter = new Intl.DateTimeFormat('en-US', { ...options, year: 'numeric' });
            const monthFormatter = new Intl.DateTimeFormat('en-US', { ...options, month: 'numeric' });
            const dayFormatter = new Intl.DateTimeFormat('en-US', { ...options, day: 'numeric' });
            const hourFormatter = new Intl.DateTimeFormat('en-US', { ...options, hour: 'numeric', hour12: false });
            const minuteFormatter = new Intl.DateTimeFormat('en-US', { ...options, minute: 'numeric' });
            
            // Extract components in the target timezone
            const year = parseInt(yearFormatter.format(date));
            const month = parseInt(monthFormatter.format(date)) - 1; // Months are 0-indexed in JS
            const day = parseInt(dayFormatter.format(date));
            
            // Handle hour format which includes AM/PM
            let hour = parseInt(hourFormatter.format(date));
            if (isNaN(hour)) {
                // Fallback if formatter returned invalid hour
                const fullTime = new Intl.DateTimeFormat('en-US', { ...options, hour: 'numeric', minute: 'numeric' }).format(date);
                const timeParts = fullTime.split(':');
                hour = parseInt(timeParts[0]);
                
                // Handle AM/PM
                if (fullTime.toLowerCase().includes('pm') && hour < 12) {
                    hour += 12;
                } else if (fullTime.toLowerCase().includes('am') && hour === 12) {
                    hour = 0;
                }
            }
            
            const minute = parseInt(minuteFormatter.format(date));
            
            // Create new date with timezone-adjusted components
            return new Date(year, month, day, hour, minute);
        } catch (error) {
            console.error('Error converting timezone:', error);
            return date; // Return original date if conversion fails
        }
    }
});