import React from 'react';
import { render, screen } from '@testing-library/react';
import { PatentArchitect } from './PatentArchitect';

describe('PatentArchitect', () => {
    it('renders patent architect accessibly', () => {
        render(<PatentArchitect />);
        expect(screen.getByRole('heading', { name: /patent architect/i })).toBeTruthy();
        expect(screen.getByRole('table', { name: /support matrix/i })).toBeTruthy();
        expect(screen.getByRole('region', { name: /figure view/i })).toBeTruthy();
        expect(screen.getByRole('button', { name: /export patent draft/i })).toBeTruthy();
    });
});
