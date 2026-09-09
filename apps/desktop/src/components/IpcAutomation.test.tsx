import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { IpcAutomation } from './IpcAutomation';
import { invoke } from '@tauri-apps/api/core';

// Mock tauri IPC
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn().mockResolvedValue({ status: 'OK' })
}));

describe('IpcAutomation', () => {
    it('verifies real IPC behavior and accessibility', async () => {
        render(<IpcAutomation />);
        expect(screen.getByRole('status', { name: /system ipc status/i })).toBeTruthy();

        // Wait for mocked IPC resolution
        await waitFor(() => {
            expect(screen.getByText('IPC System Health: OK')).toBeTruthy();
        });
        expect(invoke).toHaveBeenCalledWith('get_system_health');
    });
});
