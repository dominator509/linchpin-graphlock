import React from 'react';
import { render, screen } from '@testing-library/react';
import { Navigation } from './Navigation';

describe('Navigation', () => {
    it('renders accessible navigation', () => {
        render(<Navigation />);
        expect(screen.getByRole('navigation', { name: /main navigation/i })).toBeTruthy();
        expect(screen.getByText('Privacy Mode: Enabled (Local Only)')).toBeTruthy();
    });
});
