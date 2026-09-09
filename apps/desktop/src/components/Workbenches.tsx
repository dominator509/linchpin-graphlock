import React, { useState } from 'react';

export function ConceptionWorkbench() {
    const [conception, setConception] = useState('');

    return (
        <section aria-labelledby="conception-heading">
            <h2 id="conception-heading">Conception Workbench</h2>
            <textarea
                aria-label="Conception draft area"
                value={conception}
                onChange={e => setConception(e.target.value)}
            />
        </section>
    );
}

export function OpportunityWorkbench() {
    return (
        <section aria-labelledby="opportunity-heading">
            <h2 id="opportunity-heading">Opportunity Workbench</h2>
            <div role="region" aria-label="Opportunity scores">
                Scores: TBD
            </div>
        </section>
    );
}

export function ResearchWorkbench() {
    return (
        <section aria-labelledby="research-heading">
            <h2 id="research-heading">Research Workbench</h2>
            <button aria-label="Start Research">Start Research</button>
        </section>
    );
}
