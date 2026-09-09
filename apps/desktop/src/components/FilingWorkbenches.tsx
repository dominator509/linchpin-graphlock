import React from 'react';

export function FilingWorkbench() {
    return (
        <section aria-labelledby="filing-heading">
            <h2 id="filing-heading">Filing Workbench</h2>
            <button aria-label="Prepare Filing">Prepare Filing</button>
        </section>
    );
}

export function DocketWorkbench() {
    return (
        <section aria-labelledby="docket-heading">
            <h2 id="docket-heading">Docket Workbench</h2>
            <div role="region" aria-label="Deadlines">Deadlines Area</div>
        </section>
    );
}

export function ProsecutionWorkbench() {
    return (
        <section aria-labelledby="prosecution-heading">
            <h2 id="prosecution-heading">Prosecution Workbench</h2>
            <div role="region" aria-label="Office Actions">Office Actions Area</div>
        </section>
    );
}

export function CommercializationWorkbench() {
    return (
        <section aria-labelledby="commercial-heading">
            <h2 id="commercial-heading">Commercialization Workbench</h2>
            <div role="region" aria-label="Valuation Metrics">Valuation Metrics Area</div>
        </section>
    );
}
