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
  const [activeSection, setActiveSection] = useState('overview');

  const navLinks = [
    { id: 'overview', label: 'Overview' },
    { id: 'installation', label: 'Installation & Setup' },
    { id: 'connecting', label: 'Connecting Clients' },
    { id: 'datatypes', label: 'Data Types & Commands' },
    { id: 'advanced', label: 'Advanced Features' },
  ];

  return (
    <div className="max-w-[1400px] mx-auto px-4 sm:px-8 pt-24 pb-24 min-h-screen flex flex-col lg:flex-row gap-8">
      {/* Sidebar Navigation */}
      <aside className="lg:w-64 flex-shrink-0">
        <div className="sticky top-24 glass-effect p-6 rounded-2xl hidden lg:block">
          <h3 className="font-bold text-lg mb-4 text-white">Contents</h3>
          <ul className="space-y-3 text-sm">
            {navLinks.map(link => (
              <li key={link.id}>
                <a 
                  href={`#${link.id}`} 
                  className="text-brand-muted hover:text-brand-primary transition-colors block"
                  onClick={() => setActiveSection(link.id)}
                >
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 glass-effect p-8 lg:p-12 rounded-3xl animate-fade-in-up w-full max-w-4xl">
        <div className="flex items-center gap-4 mb-10">
          <div className="p-3 bg-brand-primary/20 rounded-xl text-brand-primary"><TerminalSquare size={32}/></div>
          <h1 className="text-4xl font-extrabold text-white">Kivo Documentation</h1>
        </div>
        
        <div className="space-y-16 text-brand-muted text-lg leading-relaxed">
          
          {/* Overview */}
          <section id="overview" className="scroll-mt-32">
            <h2 className="text-2xl font-bold text-white mb-4 border-b border-white/10 pb-2">Overview</h2>
            <p className="mb-4">
              Kivo is a high-performance, in-memory key-value store built entirely in Rust. It utilizes the powerful <strong>Tokio</strong> async runtime and features a unique <strong>16-shard lock striping</strong> architecture. This allows multiple clients to perform operations in parallel without encountering the global lock bottleneck traditionally found in single-threaded systems like Redis.
            </p>
            <p>
              Importantly, Kivo speaks the native <strong>Redis Serialization Protocol (RESP)</strong>. This means you do not need a custom client; you can drop Kivo into your existing stack and it will immediately work with <code>redis-cli</code>, <code>ioredis</code>, <code>redis-py</code>, or any other standard driver.
            </p>
          </section>

          {/* Installation */}
          <section id="installation" className="scroll-mt-32">
            <h2 className="text-2xl font-bold text-white mb-4 border-b border-white/10 pb-2">Installation & Setup</h2>
            <p className="mb-4">Building Kivo requires a stable Rust toolchain (≥ 1.75). It compiles to a single native binary.</p>
            <div className="bg-[#0a0a0f] p-6 rounded-xl font-mono text-sm border border-white/10 shadow-inner mb-6">
              <div className="text-slate-500 mb-2"># 1. Clone the repository</div>
              <div className="mb-2 text-slate-300"><span className="text-brand-primary">git clone</span> https://github.com/Nikhil-Madaravena/Kivo-InMemory-Database.git</div>
              <div className="mb-4 text-slate-300"><span className="text-brand-primary">cd</span> Kivo-InMemory-Database</div>
              <div className="text-slate-500 mb-2"># 2. Build for production</div>
              <div className="mb-4 text-slate-300"><span className="text-brand-primary">cargo build</span> --release</div>
              <div className="text-slate-500 mb-2"># 3. Start the server</div>
              <div className="text-slate-300"><span className="text-brand-primary">./target/release/kivo</span> --port 6379 --max-keys 100000</div>
            </div>

            <h3 className="text-xl font-bold text-white mt-8 mb-3">CLI Configuration Options</h3>
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm text-brand-muted border-collapse">
                <thead>
                  <tr className="border-b border-white/10 text-slate-300">
                    <th className="py-3 px-4">Flag</th>
                    <th className="py-3 px-4">Default</th>
                    <th className="py-3 px-4">Description</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/5">
                  <tr>
                    <td className="py-3 px-4 font-mono text-brand-tertiary">--port -p</td>
                    <td className="py-3 px-4 font-mono">6379</td>
                    <td className="py-3 px-4">The port number the server listens on.</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4 font-mono text-brand-tertiary">--host</td>
                    <td className="py-3 px-4 font-mono">127.0.0.1</td>
                    <td className="py-3 px-4">IP address to bind the socket to.</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4 font-mono text-brand-tertiary">--password</td>
                    <td className="py-3 px-4 font-mono">"secret"</td>
                    <td className="py-3 px-4">Set empty ("") to disable AUTH requirement.</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4 font-mono text-brand-tertiary">--max-keys</td>
                    <td className="py-3 px-4 font-mono">10000</td>
                    <td className="py-3 px-4">Maximum number of keys across all shards before LRU eviction triggers.</td>
                  </tr>
                  <tr>
                    <td className="py-3 px-4 font-mono text-brand-tertiary">--db-file</td>
                    <td className="py-3 px-4 font-mono">data.json</td>
                    <td className="py-3 px-4">Path to the background persistence snapshot file.</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          {/* Connecting Clients */}
          <section id="connecting" className="scroll-mt-32">
            <h2 className="text-2xl font-bold text-white mb-4 border-b border-white/10 pb-2">Connecting Clients</h2>
            <p className="mb-6">Because Kivo implements RESP, connecting to it is identical to connecting to a Redis instance.</p>
            
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-bold text-slate-200 mb-2">redis-cli (Terminal)</h3>
                <div className="bg-[#0a0a0f] p-4 rounded-xl font-mono text-sm border border-white/10 text-slate-300">
                  <span className="text-brand-primary">redis-cli</span> -h 127.0.0.1 -p 6379 -a secret
                </div>
              </div>

              <div>
                <h3 className="text-lg font-bold text-slate-200 mb-2">Node.js (ioredis)</h3>
                <div className="bg-[#0a0a0f] p-4 rounded-xl font-mono text-sm border border-white/10 text-slate-300 whitespace-pre">
<span className="text-brand-primary">const</span> Redis = <span className="text-brand-tertiary">require</span>(<span className="text-green-400">'ioredis'</span>);{'\n'}
<span className="text-brand-primary">const</span> client = <span className="text-brand-primary">new</span> Redis({'{'}{'\n'}
{'  '}host: <span className="text-green-400">'127.0.0.1'</span>,{'\n'}
{'  '}port: <span className="text-orange-400">6379</span>,{'\n'}
{'  '}password: <span className="text-green-400">'secret'</span>{'\n'}
{'}'});{'\n\n'}
<span className="text-brand-primary">await</span> client.set(<span className="text-green-400">'framework'</span>, <span className="text-green-400">'react'</span>);{'\n'}
<span className="text-brand-tertiary">console</span>.log(<span className="text-brand-primary">await</span> client.get(<span className="text-green-400">'framework'</span>)); <span className="text-slate-500">// 'react'</span>
                </div>
              </div>

              <div>
                <h3 className="text-lg font-bold text-slate-200 mb-2">Python (redis-py)</h3>
                <div className="bg-[#0a0a0f] p-4 rounded-xl font-mono text-sm border border-white/10 text-slate-300 whitespace-pre">
<span className="text-brand-primary">import</span> redis{'\n\n'}
r = redis.Redis(host=<span className="text-green-400">'127.0.0.1'</span>, port=<span className="text-orange-400">6379</span>, password=<span className="text-green-400">'secret'</span>){'\n'}
r.set(<span className="text-green-400">'language'</span>, <span className="text-green-400">'rust'</span>){'\n'}
<span className="text-brand-tertiary">print</span>(r.get(<span className="text-green-400">'language'</span>)) <span className="text-slate-500"># b'rust'</span>
                </div>
              </div>
            </div>
          </section>

          {/* Data Types */}
          <section id="datatypes" className="scroll-mt-32">
            <h2 className="text-2xl font-bold text-white mb-4 border-b border-white/10 pb-2">Data Types & Commands</h2>
            <p className="mb-6">Kivo fully supports the 5 core Redis data structures along with iteration (SCAN) variants.</p>
            
            <div className="grid gap-6">
              <div className="p-6 bg-white/5 rounded-2xl border border-white/10 hover:border-brand-primary/30 transition-colors">
                <h3 className="text-xl font-bold text-white mb-3 flex items-center gap-2"><span className="text-brand-primary">1.</span> Strings</h3>
                <p className="text-sm mb-4">The most basic data type. Supports raw strings, integers, and floats natively.</p>
                <div className="bg-[#0a0a0f] p-3 rounded-lg font-mono text-xs text-slate-300">
                  <span className="text-brand-primary">SET</span> key value [EX seconds] [PX millis] [NX|XX]<br/>
                  <span className="text-brand-primary">GET</span> key<br/>
                  <span className="text-brand-primary">INCRBY</span> key 5<br/>
                  <span className="text-brand-primary">MSET</span> k1 v1 k2 v2
                </div>
              </div>

              <div className="p-6 bg-white/5 rounded-2xl border border-white/10 hover:border-brand-primary/30 transition-colors">
                <h3 className="text-xl font-bold text-white mb-3 flex items-center gap-2"><span className="text-brand-primary">2.</span> Hash Maps</h3>
                <p className="text-sm mb-4">Maps between string fields and string values. Excellent for representing objects.</p>
                <div className="bg-[#0a0a0f] p-3 rounded-lg font-mono text-xs text-slate-300">
                  <span className="text-brand-primary">HSET</span> user:1000 name "John" age 30<br/>
                  <span className="text-brand-primary">HGET</span> user:1000 name<br/>
                  <span className="text-brand-primary">HGETALL</span> user:1000<br/>
                  <span className="text-brand-primary">HSCAN</span> user:1000 0 MATCH *a* COUNT 10
                </div>
              </div>

              <div className="p-6 bg-white/5 rounded-2xl border border-white/10 hover:border-brand-primary/30 transition-colors">
                <h3 className="text-xl font-bold text-white mb-3 flex items-center gap-2"><span className="text-brand-primary">3.</span> Lists</h3>
                <p className="text-sm mb-4">Linked-list style arrays. Supports fast push/pop operations from both head and tail.</p>
                <div className="bg-[#0a0a0f] p-3 rounded-lg font-mono text-xs text-slate-300">
                  <span className="text-brand-primary">LPUSH</span> mylist "world"<br/>
                  <span className="text-brand-primary">RPUSH</span> mylist "hello"<br/>
                  <span className="text-brand-primary">LRANGE</span> mylist 0 -1<br/>
                  <span className="text-brand-primary">LTRIM</span> mylist 0 99
                </div>
              </div>

              <div className="p-6 bg-white/5 rounded-2xl border border-white/10 hover:border-brand-primary/30 transition-colors">
                <h3 className="text-xl font-bold text-white mb-3 flex items-center gap-2"><span className="text-brand-primary">4.</span> Sets & Sorted Sets (ZSet)</h3>
                <p className="text-sm mb-4">Unique collections. Sorted Sets attach a floating point score to each element for automatic sorting.</p>
                <div className="bg-[#0a0a0f] p-3 rounded-lg font-mono text-xs text-slate-300">
                  <span className="text-brand-primary">SADD</span> myset "a" "b" "c"<br/>
                  <span className="text-brand-primary">ZADD</span> leaderboard 100 "player1" 250 "player2"<br/>
                  <span className="text-brand-primary">ZRANGEBYSCORE</span> leaderboard 100 300<br/>
                  <span className="text-brand-primary">ZPOPMAX</span> leaderboard 1
                </div>
              </div>
            </div>
          </section>

          {/* Advanced Features */}
          <section id="advanced" className="scroll-mt-32">
            <h2 className="text-2xl font-bold text-white mb-4 border-b border-white/10 pb-2">Advanced Features</h2>
            
            <div className="mb-8">
              <h3 className="text-xl font-bold text-white mb-2">Transactions (MULTI / EXEC)</h3>
              <p className="mb-4 text-sm">Kivo supports transactional block execution. Commands queued between MULTI and EXEC are executed atomically.</p>
              <div className="bg-[#0a0a0f] p-4 rounded-xl font-mono text-sm border border-white/10 text-slate-300">
                <span className="text-brand-primary">MULTI</span><br/>
                <span className="text-brand-primary">INCR</span> counter1<br/>
                <span className="text-brand-primary">INCR</span> counter2<br/>
                <span className="text-brand-primary">EXEC</span>
              </div>
            </div>

            <div>
              <h3 className="text-xl font-bold text-white mb-2">Keyspace Iteration (SCAN)</h3>
              <p className="mb-4 text-sm">Do not use <code>KEYS *</code> in production. Use cursor-based iteration to traverse the dataset safely without blocking other clients.</p>
              <div className="bg-[#0a0a0f] p-4 rounded-xl font-mono text-sm border border-white/10 text-slate-300">
                <span className="text-slate-500"># Syntax: SCAN cursor [MATCH pattern] [COUNT count] [TYPE type]</span><br/>
                <span className="text-brand-primary">SCAN</span> 0 MATCH user:* TYPE hash COUNT 100
              </div>
            </div>
          </section>

        </div>
      </main>
    </div>
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
