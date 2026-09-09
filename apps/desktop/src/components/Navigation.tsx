import React from 'react';

export function Navigation() {
    return (
        <nav aria-label="Main Navigation" className="main-nav">
            <ul>
                <li><a href="#workspace" aria-current="page">Workspace</a></li>
                <li><a href="#privacy">Privacy</a></li>
            </ul>
            <div className="privacy-indicator" role="status" aria-live="polite">
                Privacy Mode: Enabled (Local Only)
            </div>
        </nav>
    );
}
