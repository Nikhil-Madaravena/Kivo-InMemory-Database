import React, { useState, useEffect, useRef } from 'react';
import { Database, Zap, Layers, Lock, Terminal, Activity, ArrowRight } from 'lucide-react';
import './index.css';

// --- Background Mesh Component ---
const BackgroundMesh = () => {
  return (
    <div style={{
      position: 'fixed', top: 0, left: 0, width: '100vw', height: '100vh', 
      zIndex: -1, overflow: 'hidden', 
      background: 'radial-gradient(circle at 50% 0%, #15152a 0%, var(--bg-color) 70%)'
    }}>
      <div style={{
        position: 'absolute', top: '-10%', left: '-10%', width: '500px', height: '500px',
        borderRadius: '50%', background: 'var(--accent-primary)', filter: 'blur(80px)', opacity: 0.4,
        animation: 'float 20s infinite ease-in-out alternate'
      }} />
      <div style={{
        position: 'absolute', top: '40%', right: '-20%', width: '600px', height: '600px',
        borderRadius: '50%', background: 'var(--accent-secondary)', filter: 'blur(80px)', opacity: 0.4,
        animation: 'float-reverse 25s infinite ease-in-out alternate'
      }} />
      <div style={{
        position: 'absolute', bottom: '-20%', left: '20%', width: '400px', height: '400px',
        borderRadius: '50%', background: 'var(--accent-tertiary)', filter: 'blur(80px)', opacity: 0.3,
        animation: 'float 18s infinite ease-in-out alternate-reverse'
      }} />
    </div>
  );
};

// --- Navbar Component ---
const Navbar = () => {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 50);
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  return (
    <nav style={{
      position: 'fixed', top: 0, width: '100%', zIndex: 100, padding: '1rem 0',
      transition: 'all 0.3s ease',
      background: scrolled ? 'rgba(255, 255, 255, 0.05)' : 'transparent',
      boxShadow: scrolled ? '0 4px 30px rgba(0, 0, 0, 0.1)' : 'none',
      borderBottom: scrolled ? '1px solid var(--glass-border)' : 'none',
      backdropFilter: scrolled ? 'blur(16px)' : 'none',
      WebkitBackdropFilter: scrolled ? 'blur(16px)' : 'none'
    }}>
      <div className="container" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 800, fontSize: '1.5rem', letterSpacing: '-0.05em' }}>
          <Database color="var(--accent-primary)" />
          <span>Kivo</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '2rem' }}>
          <a href="#features" style={{ color: 'var(--text-secondary)', textDecoration: 'none', fontWeight: 500, transition: 'color 0.3s' }} onMouseOver={e => e.target.style.color = 'var(--text-primary)'} onMouseOut={e => e.target.style.color = 'var(--text-secondary)'}>Features</a>
          <a href="#installation" style={{ color: 'var(--text-secondary)', textDecoration: 'none', fontWeight: 500, transition: 'color 0.3s' }} onMouseOver={e => e.target.style.color = 'var(--text-primary)'} onMouseOut={e => e.target.style.color = 'var(--text-secondary)'}>Installation</a>
          <a href="https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database" target="_blank" rel="noreferrer" className="btn btn-outline">
            <svg height="18" width="18" viewBox="0 0 16 16" fill="currentColor">
                <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path>
            </svg> GitHub
          </a>
        </div>
      </div>
    </nav>
  );
};

// --- Terminal Mock Component ---
const TerminalMock = () => {
  const [lines, setLines] = useState(0);
  const ref = useRef(null);

  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting && lines === 0) {
        let current = 0;
        const interval = setInterval(() => {
          if (current < 8) {
            current++;
            setLines(current);
          } else {
            clearInterval(interval);
          }
        }, 300);
        observer.unobserve(entry.target);
      }
    }, { threshold: 0.5 });
    
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, [lines]);

  const terminalLines = [
    <div key={1} style={{ color: '#4ade80' }}>$ kivo --port 6379 --max-keys 50000</div>,
    <div key={2} style={{ color: '#94a3b8' }}>╔════════════════════════════════════════╗</div>,
    <div key={3} style={{ color: '#94a3b8' }}>║             kivo  v0.2.0               ║</div>,
    <div key={4} style={{ color: '#94a3b8' }}>║  Redis-compatible key-value server     ║</div>,
    <div key={5} style={{ color: '#94a3b8' }}>╚════════════════════════════════════════╝</div>,
    <div key={6} style={{ color: '#94a3b8' }}>  Listening on  : 127.0.0.1:6379</div>,
    <div key={7} style={{ color: '#94a3b8' }}>  Auth          : enabled</div>,
    <div key={8} style={{ color: '#94a3b8', marginTop: '1rem' }}>[kivo] Ready to accept connections.</div>
  ];

  return (
    <div ref={ref} className="glass-effect" style={{ borderRadius: '12px', overflow: 'hidden', position: 'relative', boxShadow: '0 20px 40px rgba(0,0,0,0.4), 0 0 0 1px var(--glass-border)' }}>
      <div style={{ position: 'absolute', top: '-50%', left: '-50%', width: '200%', height: '200%', background: 'radial-gradient(circle at center, var(--glass-glow) 0%, transparent 60%)', opacity: 0.5, pointerEvents: 'none' }} />
      <div style={{ background: 'rgba(0,0,0,0.3)', padding: '0.75rem 1rem', display: 'flex', alignItems: 'center', borderBottom: '1px solid var(--glass-border)' }}>
        <div style={{ display: 'flex', gap: '6px' }}>
          <span style={{ width: '12px', height: '12px', borderRadius: '50%', background: '#ff5f56' }} />
          <span style={{ width: '12px', height: '12px', borderRadius: '50%', background: '#ffbd2e' }} />
          <span style={{ width: '12px', height: '12px', borderRadius: '50%', background: '#27c93f' }} />
        </div>
        <div style={{ flex: 1, textAlign: 'center', fontSize: '0.85rem', color: 'var(--text-secondary)', fontFamily: '"Fira Code", monospace' }}>bash</div>
      </div>
      <div style={{ padding: '1.5rem', background: 'rgba(10, 10, 15, 0.6)', fontFamily: '"Fira Code", monospace', fontSize: '0.9rem', color: '#e2e8f0', minHeight: '260px' }}>
        {terminalLines.slice(0, lines).map((line, i) => (
          <div key={i} style={{ animation: 'typeTerminal 0.3s ease-out forwards' }}>{line}</div>
        ))}
        {lines < 8 && lines > 0 && <span style={{ display: 'inline-block', width: '8px', height: '16px', background: '#94a3b8', animation: 'blink 1s step-end infinite' }} />}
      </div>
    </div>
  );
};

// --- Feature Card Component ---
const FeatureCard = ({ icon: Icon, title, description, delay }) => {
  const [isVisible, setIsVisible] = useState(false);
  const ref = useRef(null);

  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        setIsVisible(true);
        observer.unobserve(entry.target);
      }
    }, { threshold: 0.1 });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);

  const handleMouseMove = (e) => {
    if (!ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    ref.current.style.background = `radial-gradient(circle at ${x}px ${y}px, rgba(138, 43, 226, 0.1) 0%, rgba(255, 255, 255, 0.03) 50%)`;
  };

  const handleMouseLeave = () => {
    if (ref.current) ref.current.style.background = 'rgba(255, 255, 255, 0.03)';
  };

  return (
    <div 
      ref={ref}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      className="glass-effect"
      style={{
        padding: '2rem', borderRadius: '16px', transition: 'all 0.4s cubic-bezier(0.2, 0.8, 0.2, 1)',
        opacity: isVisible ? 1 : 0, transform: isVisible ? 'translateY(0)' : 'translateY(20px)',
        transitionDelay: `${delay}s`, cursor: 'default'
      }}
      onMouseOver={e => { e.currentTarget.style.transform = 'translateY(-5px)'; e.currentTarget.style.boxShadow = '0 10px 30px rgba(0,0,0,0.2)'; e.currentTarget.style.borderColor = 'rgba(138,43,226,0.3)'; }}
      onMouseOut={e => { e.currentTarget.style.transform = 'translateY(0)'; e.currentTarget.style.boxShadow = '0 4px 30px rgba(0,0,0,0.1)'; e.currentTarget.style.borderColor = 'var(--glass-border)'; }}
    >
      <div style={{ width: '50px', height: '50px', borderRadius: '12px', background: 'rgba(138, 43, 226, 0.1)', color: '#b181ff', display: 'flex', alignItems: 'center', justifyContent: 'center', marginBottom: '1.5rem' }}>
        <Icon size={24} />
      </div>
      <h3 style={{ fontSize: '1.25rem', marginBottom: '1rem' }}>{title}</h3>
      <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>{description}</p>
    </div>
  );
};

// --- Main App Component ---
function App() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  const features = [
    { icon: Layers, title: "16-Shard Concurrency", description: "Built with 16 independent internal shards using Tokio's async runtime. Clients can operate in parallel without fighting over a global lock." },
    { icon: Terminal, title: "Redis-Compatible RESP", description: "Speaks the standard Redis Serialization Protocol. Drop it into your existing stack and it works out-of-the-box with standard Redis tools." },
    { icon: Activity, title: "Precision Expiry", description: "Supports millisecond-precision TTL constraints natively. Background tasks aggressively purge expired items to optimize memory utilization." },
    { icon: Lock, title: "Atomic Persistence", description: "Generates atomic, non-blocking JSON snapshots every configured interval. Data saves happen in the background." },
    { icon: Database, title: "Extensive Commands", description: "Supports 40+ commands across 5 core data types: Strings, Hashes, Lists, Sets, and Sorted Sets. Full suite of iteration, meta, and transaction commands." },
    { icon: Zap, title: "Intelligent LRU", description: "Pre-configured with a per-shard Least-Recently-Used eviction mechanism, ensuring that memory usage stays strictly within your max-keys boundaries." }
  ];

  return (
    <>
      <BackgroundMesh />
      <Navbar />

      <main>
        {/* Hero Section */}
        <section className="container" style={{ minHeight: '100vh', display: 'flex', alignItems: 'center', paddingTop: '5rem', gap: '4rem', flexWrap: 'wrap' }}>
          <div style={{ flex: '1 1 500px', opacity: mounted ? 1 : 0, transform: mounted ? 'translateY(0)' : 'translateY(20px)', transition: 'all 0.8s cubic-bezier(0.2, 0.8, 0.2, 1)' }}>
            <div style={{ display: 'inline-block', padding: '0.4rem 1rem', borderRadius: '100px', background: 'rgba(138, 43, 226, 0.15)', color: '#b181ff', fontSize: '0.85rem', fontWeight: 600, marginBottom: '1.5rem', border: '1px solid rgba(138, 43, 226, 0.3)' }}>
              Rust Powered
            </div>
            <h1 style={{ fontSize: 'min(4rem, 10vw)', lineHeight: 1.1, marginBottom: '1.5rem' }}>
              Lightning Fast<br />
              <span className="gradient-text">In-Memory Database</span>
            </h1>
            <p style={{ fontSize: '1.15rem', color: 'var(--text-secondary)', marginBottom: '2.5rem', maxWidth: '540px' }}>
              A highly concurrent, Redis-compatible key-value store built for performance. Featuring 16-shard lock striping, atomic background saves, and zero-compromise speed.
            </p>
            <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap' }}>
              <a href="https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database" target="_blank" rel="noreferrer" className="btn btn-primary">
                View Repository <ArrowRight size={18} />
              </a>
              <a href="#installation" className="btn btn-secondary">
                Installation Guide
              </a>
            </div>
          </div>
          <div style={{ flex: '1 1 400px', opacity: mounted ? 1 : 0, transition: 'opacity 1s ease 0.2s' }}>
            <TerminalMock />
          </div>
        </section>

        {/* Features Section */}
        <section id="features" className="container" style={{ padding: '6rem 0' }}>
          <h2 className="text-center" style={{ fontSize: '2.5rem', marginBottom: '3rem' }}>
            Uncompromising <span className="gradient-text">Architecture</span>
          </h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '2rem' }}>
            {features.map((f, i) => <FeatureCard key={i} icon={f.icon} title={f.title} description={f.description} delay={i * 0.1} />)}
          </div>
        </section>

        {/* Installation Section */}
        <section id="installation" className="container" style={{ padding: '6rem 0' }}>
          <div className="glass-effect" style={{ borderRadius: '24px', padding: 'min(4rem, 5vw)', textAlign: 'center' }}>
            <h2 style={{ fontSize: '2.5rem', marginBottom: '1rem' }}>Ready to supercharge your stack?</h2>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2.5rem' }}>Get started with Kivo in seconds. Requires Rust stable (≥ 1.75).</p>
            
            <div style={{ background: '#0d0d12', border: '1px solid var(--glass-border)', borderRadius: '12px', padding: '1.5rem', textAlign: 'left', maxWidth: '600px', margin: '0 auto', fontFamily: '"Fira Code", monospace', fontSize: '0.9rem' }}>
              <div style={{ marginBottom: '0.5rem' }}><span style={{ color: '#64748b' }}># Clone the repository</span></div>
              <div style={{ marginBottom: '0.5rem' }}><span style={{ color: 'var(--accent-primary)' }}>git clone</span> https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database.git</div>
              <div style={{ marginBottom: '0.5rem' }}><span style={{ color: 'var(--accent-primary)' }}>cd</span> Kivo-InMemory-Database</div>
              <div style={{ marginBottom: '0.5rem' }}><span style={{ color: '#64748b' }}># Build and run with release optimizations</span></div>
              <div><span style={{ color: 'var(--accent-primary)' }}>cargo run</span> --release</div>
            </div>
            
            <a href="https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database" target="_blank" rel="noreferrer" className="btn btn-primary" style={{ marginTop: '2rem' }}>
              Visit Repository <svg height="18" width="18" viewBox="0 0 16 16" fill="currentColor">
                <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path>
            </svg>
            </a>
          </div>
        </section>
      </main>

      <footer style={{ borderTop: '1px solid var(--glass-border)', padding: '3rem 0', marginTop: '4rem' }}>
        <div className="container" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem', textAlign: 'center' }}>
          <div style={{ fontWeight: 800, fontSize: '1.5rem', letterSpacing: '-0.05em' }}>
            <span className="gradient-text">Kivo</span>
          </div>
          <p style={{ color: 'var(--text-secondary)' }}>High-Performance In-Memory Key-Value Store</p>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginTop: '1rem' }}>
            &copy; {new Date().getFullYear()} Kivo Project. Open Source under MIT License.
          </div>
        </div>
      </footer>
    </>
  );
}

export default App;
