export default function App() {
  return (
    <main style={{ padding: "2rem", fontFamily: "sans-serif" }}>
      <header>
        <h1 tabIndex={0}>LINCHPIN Patent Intelligence OS</h1>
        <div role="status" aria-label="Confidentiality Indicator">
          Local-First Confidentiality Boundary Active (Device-Only)
        </div>
      </header>
      <section aria-label="Navigation Workbenches" style={{ marginTop: "1rem" }}>
        <nav>
          <button aria-label="Opportunity Radar">Opportunity Radar</button>
          <button aria-label="Human Conception Lab">Conception Lab</button>
          <button aria-label="Prior Art War Room">War Room</button>
          <button aria-label="Claim Architect">Claim Architect</button>
        </nav>
      </section>
    </main>
  );
}
