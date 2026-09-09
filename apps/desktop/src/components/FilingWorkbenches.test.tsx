import React from 'react';
import { render, screen } from '@testing-library/react';
import { FilingWorkbench, DocketWorkbench, ProsecutionWorkbench, CommercializationWorkbench } from './FilingWorkbenches';

describe('Filing and Lifecycle Workbenches', () => {
    it('renders filing workbench accessibly', () => {
        render(<FilingWorkbench />);
        expect(screen.getByRole('heading', { name: /filing workbench/i })).toBeTruthy();
        expect(screen.getByRole('button', { name: /prepare filing/i })).toBeTruthy();
    });

    it('renders docket workbench accessibly', () => {
        render(<DocketWorkbench />);
        expect(screen.getByRole('heading', { name: /docket workbench/i })).toBeTruthy();
        expect(screen.getByRole('region', { name: /deadlines/i })).toBeTruthy();
    });

    it('renders prosecution workbench accessibly', () => {
        render(<ProsecutionWorkbench />);
        expect(screen.getByRole('heading', { name: /prosecution workbench/i })).toBeTruthy();
        expect(screen.getByRole('region', { name: /office actions/i })).toBeTruthy();
    });

    it('renders commercialization workbench accessibly', () => {
        render(<CommercializationWorkbench />);
        expect(screen.getByRole('heading', { name: /commercialization workbench/i })).toBeTruthy();
        expect(screen.getByRole('region', { name: /valuation metrics/i })).toBeTruthy();
    });
});
