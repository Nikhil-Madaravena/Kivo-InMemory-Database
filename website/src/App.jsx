import React, { useState, useEffect, useRef } from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { Database, Zap, Layers, Lock, Terminal, Activity, ArrowRight, BookOpen, Code, TerminalSquare } from 'lucide-react';

const BackgroundMesh = () => (
  <div className="fixed inset-0 w-screen h-screen -z-10 overflow-hidden" style={{ background: 'radial-gradient(circle at 50% 0%, #15152a 0%, #0B0B14 70%)' }}>
    <div className="absolute top-[-10%] left-[-10%] w-[500px] h-[500px] rounded-full bg-brand-primary blur-[80px] opacity-40 animate-float" />
    <div className="absolute top-[40%] right-[-20%] w-[600px] h-[600px] rounded-full bg-brand-secondary blur-[80px] opacity-40 animate-float-reverse" />
    <div className="absolute bottom-[-20%] left-[20%] w-[400px] h-[400px] rounded-full bg-brand-tertiary blur-[80px] opacity-30 animate-float-fast" />
  </div>
);

const Navbar = () => {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 50);
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  return (
    <nav className={`fixed top-0 w-full z-50 py-4 transition-all duration-300 ${scrolled ? 'bg-white/5 backdrop-blur-md shadow-lg border-b border-white/10' : 'bg-transparent'}`}>
      <div className="max-w-7xl mx-auto px-8 flex justify-between items-center">
        <Link to="/" className="flex items-center gap-2 font-extrabold text-2xl tracking-tight">
          <Database className="text-brand-primary" />
          <span>Kivo</span>
        </Link>
        <div className="flex items-center gap-8">
          <Link to="/#features" className="text-brand-muted hover:text-brand-text font-medium transition-colors">Features</Link>
          <Link to="/docs" className="text-brand-muted hover:text-brand-text font-medium transition-colors">Documentation</Link>
          <a href="https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database" target="_blank" rel="noreferrer" className="btn btn-outline py-2">
            <svg height="18" width="18" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg> GitHub
          </a>
        </div>
      </div>
    </nav>
  );
};

const TerminalMock = () => {
  const [lines, setLines] = useState(0);
  const ref = useRef(null);

  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting && lines === 0) {
        let current = 0;
        const interval = setInterval(() => {
          if (current < 8) { current++; setLines(current); } else { clearInterval(interval); }
        }, 300);
        observer.unobserve(entry.target);
      }
    }, { threshold: 0.5 });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, [lines]);

  const terminalLines = [
    <div key={1} className="text-green-400">$ kivo --port 6379 --max-keys 50000</div>,
    <div key={2} className="text-slate-400">╔════════════════════════════════════════╗</div>,
    <div key={3} className="text-slate-400">║             kivo  v0.2.0               ║</div>,
    <div key={4} className="text-slate-400">║  Redis-compatible key-value server     ║</div>,
    <div key={5} className="text-slate-400">╚════════════════════════════════════════╝</div>,
    <div key={6} className="text-slate-400">  Listening on  : 127.0.0.1:6379</div>,
    <div key={7} className="text-slate-400">  Auth          : enabled</div>,
    <div key={8} className="text-slate-400 mt-4">[kivo] Ready to accept connections.</div>
  ];

  return (
    <div ref={ref} className="glass-effect rounded-xl overflow-hidden relative shadow-2xl">
      <div className="absolute top-[-50%] left-[-50%] w-[200%] h-[200%] bg-[radial-gradient(circle_at_center,rgba(138,43,226,0.15)_0%,transparent_60%)] opacity-50 pointer-events-none" />
      <div className="bg-black/30 px-4 py-3 flex items-center border-b border-white/10">
        <div className="flex gap-2">
          <span className="w-3 h-3 rounded-full bg-red-500" />
          <span className="w-3 h-3 rounded-full bg-yellow-400" />
          <span className="w-3 h-3 rounded-full bg-green-500" />
        </div>
        <div className="flex-1 text-center text-sm text-brand-muted font-mono">bash</div>
      </div>
      <div className="p-6 bg-[#0a0a0f]/60 font-mono text-sm text-slate-200 min-h-[260px]">
        {terminalLines.slice(0, lines).map((line, i) => (
          <div key={i} className="animate-type-terminal">{line}</div>
        ))}
        {lines < 8 && lines > 0 && <span className="inline-block w-2 h-4 bg-slate-400 animate-blink" />}
      </div>
    </div>
  );
};

const FeatureCard = ({ icon: Icon, title, description, delay }) => {
  const [isVisible, setIsVisible] = useState(false);
  const ref = useRef(null);

  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) { setIsVisible(true); observer.unobserve(entry.target); }
    }, { threshold: 0.1 });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);

  const handleMouseMove = (e) => {
    if (!ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    ref.current.style.background = `radial-gradient(circle at ${x}px ${y}px, rgba(138,43,226,0.1) 0%, rgba(255,255,255,0.03) 50%)`;
  };

  return (
    <div 
      ref={ref}
      onMouseMove={handleMouseMove}
      onMouseLeave={() => ref.current && (ref.current.style.background = 'rgba(255,255,255,0.03)')}
      className={`glass-effect p-8 rounded-2xl transition-all duration-500 hover:-translate-y-2 hover:shadow-2xl hover:border-brand-primary/30 cursor-default ${isVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'}`}
      style={{ transitionDelay: `${delay}s` }}
    >
      <div className="w-14 h-14 rounded-xl bg-brand-primary/10 text-brand-primary flex items-center justify-center mb-6">
        <Icon size={28} />
      </div>
      <h3 className="text-xl font-bold mb-4">{title}</h3>
      <p className="text-brand-muted leading-relaxed">{description}</p>
    </div>
  );
};

const LandingPage = () => {
  const [mounted, setMounted] = useState(false);
  const location = useLocation();

  useEffect(() => {
    setMounted(true);
    if (location.hash === '#features') {
      const el = document.getElementById('features');
      if (el) el.scrollIntoView({ behavior: 'smooth' });
    } else {
      window.scrollTo(0, 0);
    }
  }, [location]);

  const features = [
    { icon: Layers, title: "16-Shard Concurrency", description: "Built with 16 independent internal shards using Tokio's async runtime. Clients can operate in parallel without fighting over a global lock." },
    { icon: Terminal, title: "Redis-Compatible RESP", description: "Speaks the standard Redis Serialization Protocol. Drop it into your existing stack and it works out-of-the-box with standard Redis tools." },
    { icon: Activity, title: "Precision Expiry", description: "Supports millisecond-precision TTL constraints natively. Background tasks aggressively purge expired items to optimize memory utilization." },
    { icon: Lock, title: "Atomic Persistence", description: "Generates atomic, non-blocking JSON snapshots every configured interval. Data saves happen in the background." },
    { icon: Database, title: "Extensive Commands", description: "Supports 40+ commands across 5 core data types: Strings, Hashes, Lists, Sets, and Sorted Sets. Full suite of iteration, meta, and transaction commands." },
    { icon: Zap, title: "Intelligent LRU", description: "Pre-configured with a per-shard Least-Recently-Used eviction mechanism, ensuring that memory usage stays strictly within your max-keys boundaries." }
  ];

  return (
    <main>
      <section className="max-w-7xl mx-auto px-8 min-h-screen flex items-center pt-20 gap-16 flex-wrap">
        <div className={`flex-1 min-w-[300px] transition-all duration-1000 ${mounted ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'}`}>
          <div className="inline-block px-4 py-1.5 rounded-full bg-brand-primary/15 text-brand-primary border border-brand-primary/30 text-sm font-semibold mb-6">
            Rust Powered
          </div>
          <h1 className="text-6xl lg:text-7xl font-extrabold leading-tight mb-6">
            Lightning Fast<br />
            <span className="gradient-text">In-Memory Database</span>
          </h1>
          <p className="text-xl text-brand-muted mb-10 max-w-xl leading-relaxed">
            A highly concurrent, Redis-compatible key-value store built for performance. Featuring 16-shard lock striping, atomic background saves, and zero-compromise speed.
          </p>
          <div className="flex gap-4 flex-wrap">
            <Link to="/docs" className="btn btn-primary">
              <BookOpen size={20} /> Read Documentation
            </Link>
            <a href="https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database" className="btn btn-secondary">
              <Code size={20} /> View Source
            </a>
          </div>
        </div>
        <div className={`flex-1 min-w-[300px] transition-all duration-1000 delay-200 ${mounted ? 'opacity-100' : 'opacity-0'}`}>
          <TerminalMock />
        </div>
      </section>

      <section id="features" className="max-w-7xl mx-auto px-8 py-32">
        <h2 className="text-center text-4xl lg:text-5xl font-extrabold mb-16">
          Uncompromising <span className="gradient-text">Architecture</span>
        </h2>
        <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
          {features.map((f, i) => <FeatureCard key={i} {...f} delay={i * 0.1} />)}
        </div>
      </section>
    </main>
  );
};

const DocumentationPage = () => {
  useEffect(() => { window.scrollTo(0, 0); }, []);
  
  return (
    <main className="max-w-4xl mx-auto px-8 pt-32 pb-24 min-h-screen">
      <div className="glass-effect p-12 rounded-3xl animate-fade-in-up">
        <div className="flex items-center gap-4 mb-8">
          <div className="p-3 bg-brand-primary/20 rounded-xl text-brand-primary"><TerminalSquare size={32}/></div>
          <h1 className="text-4xl font-extrabold">Kivo Documentation</h1>
        </div>
        
        <div className="space-y-12 text-brand-muted text-lg leading-relaxed">
          <section>
            <h2 className="text-2xl font-bold text-white mb-4">Installation</h2>
            <p className="mb-4">Getting started with Kivo is straightforward. Ensure you have Rust stable (≥ 1.75) installed.</p>
            <div className="bg-[#0a0a0f] p-6 rounded-xl font-mono text-sm border border-white/10">
              <div className="text-slate-400 mb-2"># Clone the repository</div>
              <div className="mb-2"><span className="text-brand-primary">git clone</span> https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database.git</div>
              <div className="mb-2"><span className="text-brand-primary">cd</span> Kivo-InMemory-Database</div>
              <div className="text-slate-400 mt-4 mb-2"># Build and run</div>
              <div><span className="text-brand-primary">cargo run</span> --release</div>
            </div>
          </section>

          <section>
            <h2 className="text-2xl font-bold text-white mb-4">CLI Options</h2>
            <p className="mb-4">Configure Kivo exactly to your needs directly from the command line:</p>
            <ul className="list-disc pl-6 space-y-2">
              <li><code className="text-brand-tertiary bg-brand-tertiary/10 px-2 py-1 rounded">--port &lt;PORT&gt;</code> - The port to listen on (default 6379).</li>
              <li><code className="text-brand-tertiary bg-brand-tertiary/10 px-2 py-1 rounded">--host &lt;HOST&gt;</code> - The IP address to bind to.</li>
              <li><code className="text-brand-tertiary bg-brand-tertiary/10 px-2 py-1 rounded">--password &lt;PASS&gt;</code> - Required password for AUTH. Leave empty to disable.</li>
              <li><code className="text-brand-tertiary bg-brand-tertiary/10 px-2 py-1 rounded">--max-keys &lt;NUM&gt;</code> - LRU limit for total keys across all shards.</li>
              <li><code className="text-brand-tertiary bg-brand-tertiary/10 px-2 py-1 rounded">--db-file &lt;FILE&gt;</code> - Path to background snapshot JSON file.</li>
            </ul>
          </section>
          
          <section>
            <h2 className="text-2xl font-bold text-white mb-4">Supported Data Types</h2>
            <div className="grid sm:grid-cols-2 gap-6 mt-6">
              <div className="p-6 bg-white/5 rounded-2xl border border-white/10">
                <h3 className="font-bold text-white mb-2">Strings</h3>
                <p className="text-sm">SET, GET, INCR, DECR, APPEND, MSET, MGET</p>
              </div>
              <div className="p-6 bg-white/5 rounded-2xl border border-white/10">
                <h3 className="font-bold text-white mb-2">Lists</h3>
                <p className="text-sm">LPUSH, RPUSH, LPOP, RPOP, LRANGE, LINDEX, LTRIM</p>
              </div>
              <div className="p-6 bg-white/5 rounded-2xl border border-white/10">
                <h3 className="font-bold text-white mb-2">Hashes</h3>
                <p className="text-sm">HSET, HGET, HGETALL, HDEL, HLEN, HSCAN</p>
              </div>
              <div className="p-6 bg-white/5 rounded-2xl border border-white/10">
                <h3 className="font-bold text-white mb-2">Sets & Sorted Sets</h3>
                <p className="text-sm">SADD, SMEMBERS, SUNION, ZADD, ZRANGE, ZSCAN</p>
              </div>
            </div>
          </section>
        </div>
      </div>
    </main>
  );
};

const Footer = () => (
  <footer className="border-t border-white/10 py-12 mt-12 text-center">
    <div className="max-w-7xl mx-auto px-8 flex flex-col items-center gap-4">
      <div className="font-extrabold text-2xl tracking-tight">
        <span className="gradient-text">Kivo</span>
      </div>
      <p className="text-brand-muted">High-Performance In-Memory Key-Value Store</p>
      <div className="text-brand-muted/60 text-sm mt-4">
        &copy; {new Date().getFullYear()} Kivo Project. Open Source under MIT License.
      </div>
    </div>
  </footer>
);

function App() {
  return (
    <>
      <BackgroundMesh />
      <Navbar />
      <Routes>
        <Route path="/" element={<LandingPage />} />
        <Route path="/docs" element={<DocumentationPage />} />
      </Routes>
      <Footer />
    </>
  );
}

export default App;
