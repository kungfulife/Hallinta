(() => {
    const logLevelOrder = { DEV: -1, DEBUG: 0, INFO: 1, WARN: 2, ERROR: 3 };

    const formatLocalTimestamp = (utcTimestamp) => {
        if (!utcTimestamp) return '';
        try {
            const date = new Date(utcTimestamp);
            if (isNaN(date.getTime())) return utcTimestamp;
            const y = date.getFullYear();
            const mo = String(date.getMonth() + 1).padStart(2, '0');
            const d = String(date.getDate()).padStart(2, '0');
            const h = String(date.getHours()).padStart(2, '0');
            const mi = String(date.getMinutes()).padStart(2, '0');
            const s = String(date.getSeconds()).padStart(2, '0');
            return `${y}-${mo}-${d} ${h}:${mi}:${s}`;
        } catch {
            return String(utcTimestamp).replace('T', ' ').replace(/\.\d+.*$/, '');
        }
    };

    const escapeHtml = (text) => {
        const div = document.createElement('div');
        div.textContent = text ?? '';
        return div.innerHTML;
    };

    const highlightText = (html, searchTerm) => {
        if (!searchTerm) return html;
        const escapedTerm = searchTerm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        const regex = new RegExp(`(${escapedTerm})`, 'gi');
        return html.replace(regex, '<mark class="log-highlight">$1</mark>');
    };

    const initLogFilterDropdown = (selectIds, currentLogLevel) => {
        const currentOrdinal = logLevelOrder[currentLogLevel] ?? 1;
        selectIds.forEach(id => {
            const select = document.getElementById(id);
            if (!select) return;
            select.value = currentLogLevel;
            for (const option of select.options) {
                option.disabled = (logLevelOrder[option.value] ?? 0) < currentOrdinal;
            }
        });
    };

    const buildLogHTML = (logs, selectedLevel, searchText) => {
        if (!Array.isArray(logs) || logs.length === 0) {
            return '<div class="log-line log-info"><span class="log-msg">No logs available.</span></div>';
        }

        const selectedOrdinal = logLevelOrder[selectedLevel] ?? 1;
        const loweredSearch = (searchText || '').toLowerCase();

        const filteredLogs = logs.filter(log => {
            const level = String(log.level || '').toUpperCase();
            if (level !== 'DEV' && (logLevelOrder[level] ?? 0) < selectedOrdinal) return false;
            if (!loweredSearch) return true;
            const text = `${log.message || ''} ${log.module || ''}`.toLowerCase();
            return text.includes(loweredSearch);
        });

        if (filteredLogs.length === 0) {
            return '<div class="log-line log-info"><span class="log-msg">No matching logs.</span></div>';
        }

        return filteredLogs.map(log => {
            const level = String(log.level || 'INFO').toUpperCase();
            const levelClass = `log-${level.toLowerCase()}`;
            const timestamp = formatLocalTimestamp(log.timestamp);
            let msgHtml = escapeHtml(log.message || '');
            if (searchText) {
                msgHtml = highlightText(msgHtml, searchText);
            }
            return `<div class="log-line ${levelClass}"><span class="log-meta">[${timestamp}] [${level}] [${log.module || ''}] </span><span class="log-msg">${msgHtml}</span></div>`;
        }).join('');
    };

    window.logUtils = {
        logLevelOrder,
        escapeHtml,
        highlightText,
        initLogFilterDropdown,
        buildLogHTML
    };
})();
