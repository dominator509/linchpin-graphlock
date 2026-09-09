import React from 'react';
import { render, screen } from '@testing-library/react';
import { ConceptionWorkbench, OpportunityWorkbench, ResearchWorkbench } from './Workbenches';

describe('Workbenches', () => {
    it('renders conception workbench accessibly', () => {
        render(<ConceptionWorkbench />);
        expect(screen.getByRole('heading', { name: /conception workbench/i })).toBeTruthy();
        expect(screen.getByRole('textbox', { name: /conception draft area/i })).toBeTruthy();
    });

    it('renders opportunity workbench accessibly', () => {
        render(<OpportunityWorkbench />);
        expect(screen.getByRole('heading', { name: /opportunity workbench/i })).toBeTruthy();
        expect(screen.getByRole('region', { name: /opportunity scores/i })).toBeTruthy();
    });

    it('renders research workbench accessibly', () => {
        render(<ResearchWorkbench />);
        expect(screen.getByRole('heading', { name: /research workbench/i })).toBeTruthy();
        expect(screen.getByRole('button', { name: /start research/i })).toBeTruthy();
    });
});
