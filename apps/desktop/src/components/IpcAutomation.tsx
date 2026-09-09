import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export function IpcAutomation() {
    const [status, setStatus] = useState('Unknown');

    useEffect(() => {
        // Keyboard and screen reader automation boundary
        invoke('get_system_health')
            .then((res: any) => setStatus(res.status))
            .catch(() => setStatus('Error'));
    }, []);

    return (
        <div role="status" aria-live="assertive" aria-label="System IPC Status">
            IPC System Health: {status}
        </div>
    );
}
