import { useState, useEffect } from "react";
import { Shield, Globe, Wallet, Settings, Power, Activity, Lock, Users, Radio, Cpu, LogOut, History as HistoryIcon, Zap } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
// import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// Types
type Tab = "home" | "map" | "wallet" | "settings";
type ConnectionStatus = "disconnected" | "connecting" | "connected";
type NodeGroup = "COMMUNITY" | "PUBLIC";

interface Node {
  id: string;
  country_code: string;
  bandwidth_mbps: number;
  latency_ms: number;
  group: string;
}

// Country coordinates mapping (approximate for the SVG map)
const COUNTRY_COORDS: Record<string, { top: string; left: string }> = {
  "US": { top: "28%", left: "15%" },
  "GB": { top: "25%", left: "47%" },
  "JP": { top: "35%", left: "85%" },
  "FR": { top: "30%", left: "49%" },
  "DE": { top: "28%", left: "51%" },
  "CA": { top: "20%", left: "18%" },
  "BR": { top: "65%", left: "32%" },
  "IN": { top: "45%", left: "70%" },
  "AU": { top: "75%", left: "85%" },
  "SG": { top: "55%", left: "80%" },
  "KR": { top: "33%", left: "84%" },
  "NL": { top: "26%", left: "49%" },
  "RU": { top: "22%", left: "65%" },
};

// Mock Data
const MOCK_SESSIONS = [
  { id: 1, country: "DE", type: "browsing", bytes: "15.4 MB", earning: "+0.15 CR" },
  { id: 2, country: "IR", type: "censorship-bypass", bytes: "42.1 MB", earning: "+0.80 CR" },
  { id: 3, country: "US", type: "streaming", bytes: "128.0 MB", earning: "+1.20 CR" },
];

interface User {
  username: string;
  credits: number;
  token: string;
}

function App() {
  const [user, setUser] = useState<User | null>(null);
  const [authMode, setAuthMode] = useState<"login" | "register">("login");
  const [authData, setAuthData] = useState({ username: "", password: "", email: "" });
  const [activeTab, setActiveTab] = useState<Tab>("home");
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");
  const [isSharing, setIsSharing] = useState(true);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [nodeGroup, setNodeGroup] = useState<NodeGroup>("COMMUNITY");
  const [traffic, setTraffic] = useState({ down: 0, up: 0 });

  // Simulate real-time traffic when connected
  useEffect(() => {
    if (status !== 'connected') {
      setTraffic({ down: 0, up: 0 });
      return;
    }
    const interval = setInterval(() => {
      setTraffic({
        down: Math.random() * 25 + 5, // 5-30 Mbps
        up: Math.random() * 5 + 1      // 1-6 Mbps
      });
    }, 2000);
    return () => clearInterval(interval);
  }, [status]);

  const handleAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    // Simulation d'appel API (on branchera le fetch réel au prochain step)
    // Pour l'instant on valide pour entrer dans l'app
    setUser({
      username: authData.username || "Explorer",
      credits: 1250,
      token: "fake-jwt-token"
    });
  };

  // Fetch nodes from backend
  useEffect(() => {
    const fetchNodes = async () => {
      try {
        // En mode dev, si le backend n'est pas prêt, on garde des données de secours
        // Real logic would be: const response = await invoke("discover_nodes", { group: nodeGroup });
        // For now we will use a mix of real call when ready and simulated data
        if (activeTab === 'map') {
          // Simulation de l'appel pour l'instant car la commande "discover_nodes" doit être ajoutée au Rust
          const mockNodes: Node[] = nodeGroup === 'PUBLIC'
            ? [
              { id: '1', country_code: 'JP', bandwidth_mbps: 100, latency_ms: 120, group: 'PUBLIC' },
              { id: '2', country_code: 'US', bandwidth_mbps: 80, latency_ms: 45, group: 'PUBLIC' },
              { id: '3', country_code: 'DE', bandwidth_mbps: 50, latency_ms: 15, group: 'PUBLIC' },
            ]
            : [
              { id: '4', country_code: 'FR', bandwidth_mbps: 20, latency_ms: 10, group: 'COMMUNITY' },
              { id: '5', country_code: 'IN', bandwidth_mbps: 15, latency_ms: 65, group: 'COMMUNITY' },
            ];
          setNodes(mockNodes);
        }
      } catch (e) {
        console.error("Failed to fetch nodes", e);
      }
    };
    fetchNodes();
    const interval = setInterval(fetchNodes, 10000);
    return () => clearInterval(interval);
  }, [activeTab, nodeGroup]);

  const toggleConnection = () => {
    if (status === "disconnected") {
      setStatus("connecting");
      setTimeout(() => setStatus("connected"), 2000);
    } else {
      setStatus("disconnected");
    }
  };

  if (!user) {
    return (
      <div className="flex h-screen w-screen bg-background items-center justify-center relative overflow-hidden p-6">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_center,_var(--tw-gradient-stops))] from-primary/10 to-background opacity-50" />

        <motion.div
          initial={{ y: 20, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          className="z-10 w-full max-w-md bg-surface/60 backdrop-blur-2xl border border-white/10 rounded-[2rem] p-10 shadow-2xl"
        >
          <div className="flex flex-col items-center mb-8">
            <div className="w-20 h-20 bg-primary/10 border border-primary/20 rounded-3xl flex items-center justify-center mb-4 shadow-[0_0_30px_rgba(0,242,234,0.1)]">
              <Shield className="w-10 h-10 text-primary" />
            </div>
            <h1 className="text-3xl font-bold text-white tracking-tighter">WorldVPN</h1>
            <p className="text-text-muted text-sm mt-2">Decentralized Secure Perimeter</p>
          </div>

          <form onSubmit={handleAuth} className="space-y-4">
            {authMode === 'register' && (
              <div className="space-y-1">
                <label className="text-[10px] font-bold text-text-muted uppercase ml-1">Email Address</label>
                <input
                  type="email"
                  className="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white focus:border-primary/50 outline-none transition-all"
                  placeholder="name@example.com"
                />
              </div>
            )}
            <div className="space-y-1">
              <label className="text-[10px] font-bold text-text-muted uppercase ml-1">Username</label>
              <input
                type="text"
                value={authData.username}
                onChange={(e) => setAuthData({ ...authData, username: e.target.value })}
                className="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white focus:border-primary/50 outline-none transition-all font-mono"
                placeholder="identity_01"
                required
              />
            </div>
            <div className="space-y-1">
              <label className="text-[10px] font-bold text-text-muted uppercase ml-1">Access Key</label>
              <input
                type="password"
                className="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white focus:border-primary/50 outline-none transition-all"
                placeholder="••••••••"
                required
              />
            </div>

            <button type="submit" className="w-full bg-primary text-background font-bold py-4 rounded-xl mt-4 hover:scale-[1.02] active:scale-[0.98] transition-all shadow-[0_0_20px_rgba(0,242,234,0.3)]">
              {authMode === 'login' ? 'INITIALIZE SESSION' : 'CREATE PROTOCOL'}
            </button>
          </form>

          <div className="mt-8 flex justify-center gap-2 text-xs">
            <span className="text-text-muted">
              {authMode === 'login' ? "Don't have an identity?" : "Already have a key?"}
            </span>
            <button
              onClick={() => setAuthMode(authMode === 'login' ? 'register' : 'login')}
              className="text-primary font-bold hover:underline"
            >
              {authMode === 'login' ? "Register Now" : "Login Here"}
            </button>
          </div>
        </motion.div>

        {/* Decorative elements */}
        <div className="absolute top-10 left-10 opacity-20 font-mono text-[10px] text-primary">
          SECURE_BOOT: ENABLED<br />V_VERSION: 1.0.4<br />ENCR: XDH_AES_GCM
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen w-screen text-text-main overflow-hidden bg-background font-sans">
      <div className="ambient-glow opacity-30" />

      {/* Navigation (Sidebar on desktop, Bottom Bar on mobile) */}
      <nav className="fixed bottom-0 left-0 right-0 h-16 md:relative md:h-full md:w-20 bg-surface/80 backdrop-blur-xl border-t md:border-t-0 md:border-r border-white/5 flex md:flex-col items-center justify-around md:justify-start md:py-8 z-30">
        <div className="hidden md:block mb-12">
          <Shield className="w-8 h-8 text-primary shadow-[0_0_15px_rgba(0,242,234,0.3)]" />
        </div>

        <div className="flex md:flex-col gap-2 md:gap-6 w-full items-center justify-around md:justify-center">
          <NavIcon icon={Globe} label="Network" active={activeTab === "home"} onClick={() => setActiveTab("home")} />
          <NavIcon icon={Users} label="Peers" active={activeTab === "map"} onClick={() => setActiveTab("map")} />
          <NavIcon icon={Wallet} label="Wallet" active={activeTab === "wallet"} onClick={() => setActiveTab("wallet")} />
          <NavIcon icon={Settings} label="Config" active={activeTab === "settings"} onClick={() => setActiveTab("settings")} />
        </div>

        <div className="hidden md:flex mt-auto flex-col gap-4 items-center">
          <div
            className={`w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer transition-all hover:bg-white/5 ${isSharing ? 'text-success' : 'text-text-muted'}`}
            onClick={() => setIsSharing(!isSharing)}
            title={isSharing ? "Sharing Enabled" : "Sharing Paused"}
          >
            <Radio className={`w-5 h-5 ${isSharing ? 'animate-pulse' : ''}`} />
          </div>

          <div className="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer text-danger hover:bg-danger/10 transition-all">
            <LogOut className="w-5 h-5" />
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <main className="flex-1 flex flex-col relative z-10">
        {/* Header */}
        <header className="h-20 flex items-center justify-between px-8 border-b border-white/5 bg-surface/30 backdrop-blur-sm">
          <div>
            <div className="flex items-center gap-3">
              <h1 className="text-xl font-bold tracking-tight text-white capitalize">{activeTab}</h1>
              <span className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider border ${status === 'connected' ? 'border-success/30 text-success bg-success/10' : 'border-text-muted/30 text-text-muted'}`}>
                {status === 'connected' ? 'Secure' : 'Unprotected'}
              </span>
            </div>
            <p className="text-xs text-text-muted font-mono mt-1 opacity-70">IP: {status === 'connected' ? '10.8.42.19 (Protected)' : '192.168.1.42 (Exposed)'}</p>
          </div>

          <div className="flex items-center gap-4">
            <div className="bg-surface-highlight/50 border border-white/5 px-4 py-2 rounded-lg flex items-center gap-3">
              <div className="bg-secondary/20 p-1.5 rounded-md">
                <Wallet className="w-4 h-4 text-secondary" />
              </div>
              <div>
                <div className="text-sm font-mono font-bold text-white leading-none">{user.credits.toLocaleString()} CR</div>
              </div>
            </div>
          </div>
        </header>

        {/* Dynamic Content Area */}
        <div className="flex-1 overflow-hidden relative">
          <AnimatePresence mode="wait">
            {activeTab === 'home' && (
              <motion.div
                key="home"
                initial={{ opacity: 0, scale: 0.98 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.98 }}
                transition={{ duration: 0.2 }}
                className="absolute inset-0 p-4 md:p-8 grid grid-cols-1 md:grid-cols-12 gap-6 overflow-y-auto md:overflow-hidden"
              >
                {/* Main Panel: Connection */}
                <div className="md:col-span-8 flex flex-col gap-6">
                  <div className="h-[300px] md:flex-1 relative rounded-3xl overflow-hidden border border-white/5 bg-surface/40 flex items-center justify-center group">

                    {/* Animated Map Background */}
                    <div className="absolute inset-0 opacity-10 bg-[url('https://upload.wikimedia.org/wikipedia/commons/e/ec/World_map_blank_without_borders.svg')] bg-cover bg-center mix-blend-overlay transition-opacity duration-1000 group-hover:opacity-20" />

                    {/* Radar Effect */}
                    {status === 'connecting' && (
                      <div className="absolute inset-0 flex items-center justify-center">
                        <div className="w-[500px] h-[500px] border border-primary/20 rounded-full animate-ping opacity-20" />
                        <div className="w-[300px] h-[300px] border border-primary/40 rounded-full animate-ping opacity-30 animation-delay-500" />
                      </div>
                    )}

                    <div className="relative z-10 flex flex-col items-center gap-8">
                      <motion.button
                        whileHover={{ scale: 1.05 }}
                        whileTap={{ scale: 0.95 }}
                        onClick={toggleConnection}
                        className={`relative w-40 h-40 rounded-full flex flex-col items-center justify-center transition-all duration-500 shadow-2xl
                          ${status === "connected"
                            ? "bg-gradient-to-br from-primary/20 to-primary/5 border-2 border-primary shadow-[0_0_60px_rgba(0,242,234,0.2)]"
                            : status === "connecting"
                              ? "bg-surface-highlight border-2 border-white/20 animate-pulse"
                              : "bg-surface border-2 border-white/10 hover:border-white/30 hover:bg-surface-highlight"
                          }`}
                      >
                        <Power className={`w-12 h-12 mb-2 transition-colors duration-500 ${status === "connected" ? "text-primary drop-shadow-[0_0_10px_rgba(0,242,234,0.8)]" : "text-text-muted"}`} />
                        <span className={`uppercase tracking-widest font-bold text-xs ${status === "connected" ? "text-primary" : "text-text-muted"}`}>
                          {status === "connected" ? "ON" : status === "connecting" ? "INIT" : "OFF"}
                        </span>
                      </motion.button>

                      <div className="h-8">
                        <AnimatePresence mode="wait">
                          {status === "connected" && (
                            <motion.div
                              initial={{ opacity: 0, y: 10 }}
                              animate={{ opacity: 1, y: 0 }}
                              exit={{ opacity: 0, y: -10 }}
                              className="flex flex-col items-center"
                            >
                              <span className="text-secondary font-bold text-lg flex items-center gap-2">
                                <Shield className="w-4 h-4" /> Tokyo, Japan
                              </span>
                              <span className="text-xs font-mono text-text-muted bg-surface-highlight px-2 py-0.5 rounded mt-1">
                                AES-256-GCM • 15ms
                              </span>
                            </motion.div>
                          )}
                          {status === "disconnected" && (
                            <motion.div
                              initial={{ opacity: 0 }}
                              animate={{ opacity: 1 }}
                              exit={{ opacity: 0 }}
                              className="text-text-muted text-sm"
                            >
                              Ready to connect
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    </div>
                  </div>

                  {/* Quick Stats Row */}
                  <div className="grid grid-cols-3 gap-4 h-32">
                    <StatCard icon={Activity} label="Latency" value={status === 'connected' ? "15 ms" : "--"} sub="Optimized" color="text-success" />
                    <StatCard icon={Zap} label="Download" value={status === 'connected' ? `${traffic.down.toFixed(1)} Mbps` : "--"} sub="Global Route" color="text-primary" />
                    <StatCard icon={Zap} label="Upload" value={status === 'connected' ? `${traffic.up.toFixed(1)} Mbps` : "--"} sub="Secure Tunnel" color="text-secondary" />
                  </div>
                </div>

                {/* Right Panel: Live Transparency */}
                <div className="col-span-4 bg-surface/30 backdrop-blur-md rounded-3xl border border-white/5 flex flex-col overflow-hidden">
                  <div className="p-6 border-b border-white/5 flex justify-between items-center bg-white/[0.02]">
                    <h2 className="font-bold flex items-center gap-2 text-sm uppercase tracking-wider">
                      <Activity className="w-4 h-4 text-secondary" />
                      Live Nodes
                    </h2>
                    <div className={`w-2 h-2 rounded-full ${isSharing ? 'bg-success shadow-[0_0_10px_#00ff9d]' : 'bg-red-500'}`} />
                  </div>

                  <div className="flex-1 overflow-y-auto p-4 space-y-3">
                    <AnimatePresence>
                      {isSharing ? (
                        <>
                          {MOCK_SESSIONS.map((session, i) => (
                            <motion.div
                              key={session.id}
                              initial={{ opacity: 0, x: 20 }}
                              animate={{ opacity: 1, x: 0 }}
                              transition={{ delay: i * 0.1 }}
                              className="bg-surface-highlight/50 hover:bg-surface-highlight transition-colors rounded-xl p-3 border border-white/5 flex items-center justify-between group"
                            >
                              <div className="flex items-center gap-3">
                                <div className="w-10 h-10 rounded-lg bg-surface flex items-center justify-center font-bold text-sm border border-white/5 text-text-muted group-hover:text-white group-hover:border-white/20 transition-all">
                                  {session.country}
                                </div>
                                <div>
                                  <div className="text-sm font-medium text-white">{session.bytes}</div>
                                  <div className="text-[10px] uppercase tracking-wide text-text-muted">{session.type}</div>
                                </div>
                              </div>
                              <div className="text-right">
                                <div className="text-secondary font-mono text-xs font-bold">{session.earning}</div>
                                <div className="text-[10px] text-text-muted">earning</div>
                              </div>
                            </motion.div>
                          ))}

                          <div className="mt-8 mx-auto w-full flex justify-center">
                            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-surface border border-white/5 text-xs text-text-muted">
                              <span className="w-2 h-2 bg-primary rounded-full animate-pulse"></span>
                              Scanning P2P Network...
                            </div>
                          </div>
                        </>
                      ) : (
                        <div className="h-full flex flex-col items-center justify-center text-center opacity-40 p-8">
                          <Lock className="w-12 h-12 mb-4 text-text-muted" />
                          <p className="text-sm font-bold">Sharing Disabled</p>
                          <p className="text-xs mt-2 text-text-muted">Enable sharing to earn credits based on bandwidth usage.</p>
                        </div>
                      )}
                    </AnimatePresence>
                  </div>
                </div>
              </motion.div>
            )}

            {activeTab === 'map' && (
              <motion.div
                key="map"
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                className="absolute inset-0 p-8 flex flex-col"
              >
                <div className="flex justify-between items-end mb-6">
                  <div>
                    <h2 className="text-2xl font-bold text-white tracking-tight">Global Infrastructure</h2>
                    <p className="text-sm text-text-muted">Currently viewing: <span className={nodeGroup === 'COMMUNITY' ? 'text-primary' : 'text-secondary'}>{nodeGroup === 'COMMUNITY' ? 'P2P Community Network' : 'Public VPN Gateways'}</span></p>
                  </div>

                  {/* Network Switcher Toggle */}
                  <div className="flex bg-surface-highlight/40 p-1 rounded-xl border border-white/5">
                    <button
                      onClick={() => setNodeGroup('COMMUNITY')}
                      className={`px-4 py-1.5 rounded-lg text-xs font-bold transition-all ${nodeGroup === 'COMMUNITY' ? 'bg-primary text-background shadow-[0_0_10px_rgba(0,242,234,0.5)]' : 'text-text-muted hover:text-white'}`}
                    >
                      Community
                    </button>
                    <button
                      onClick={() => setNodeGroup('PUBLIC')}
                      className={`px-4 py-1.5 rounded-lg text-xs font-bold transition-all ${nodeGroup === 'PUBLIC' ? 'bg-secondary text-white shadow-[0_0_10px_rgba(112,0,255,0.5)]' : 'text-text-muted hover:text-white'}`}
                    >
                      Public Gate
                    </button>
                  </div>
                </div>

                <div className="flex-1 bg-surface/30 border border-white/5 rounded-3xl relative overflow-hidden flex items-center justify-center p-12 group">
                  {/* World Map SVG */}
                  <div className="relative w-full max-w-4xl opacity-40 group-hover:opacity-60 transition-opacity duration-1000 grayscale group-hover:grayscale-0">
                    <img
                      src="https://upload.wikimedia.org/wikipedia/commons/e/ec/World_map_blank_without_borders.svg"
                      alt="World Map"
                      className="w-full h-auto filter invert brightness-150"
                    />

                    {/* Dynamic Nodes from API */}
                    {nodes.map((node) => {
                      const coords = COUNTRY_COORDS[node.country_code] || { top: "50%", left: "50%" };
                      return (
                        <MapNode
                          key={node.id}
                          top={coords.top}
                          left={coords.left}
                          label={`${node.country_code} Server`}
                          latency={`${node.latency_ms}ms`}
                          active={nodeGroup === 'COMMUNITY'}
                        />
                      );
                    })}
                  </div>

                  {/* Scanning HUD Overlay */}
                  <div className="absolute inset-0 pointer-events-none border border-primary/10 m-4 rounded-2xl">
                    <div className="absolute top-0 left-0 p-4 font-mono text-[10px] text-primary/40 leading-relaxed uppercase">
                      Network: {nodeGroup}<br />
                      Status: Fetching Topology...<br />
                      Nodes_Count: {nodes.length}
                    </div>
                    <div className="absolute bottom-0 right-0 p-4 font-mono text-[10px] text-secondary/40 text-right uppercase">
                      Encryption: Multi-Layer<br />
                      Layer: Decentralized_Grid
                    </div>
                  </div>
                </div>
              </motion.div>
            )}

            {activeTab === 'wallet' && (
              <motion.div
                key="wallet"
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                className="absolute inset-0 p-8 overflow-y-auto"
              >
                <div className="flex justify-between items-center mb-6">
                  <h2 className="text-2xl font-bold text-white tracking-tight">Wallet & Rewards</h2>
                  <motion.button
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    className="btn-primary text-sm py-2 px-6"
                  >
                    Withdraw Credits
                  </motion.button>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  {/* Balance Card */}
                  <div className="bg-gradient-to-br from-surface/80 to-surface-highlight/40 border border-white/10 rounded-3xl p-8 relative overflow-hidden group">
                    <div className="absolute top-0 right-0 p-6 opacity-10 group-hover:opacity-20 transition-opacity">
                      <Wallet className="w-24 h-24 text-secondary rotate-12" />
                    </div>
                    <div className="relative z-10">
                      <div className="text-text-muted text-xs font-bold uppercase tracking-widest mb-1 opacity-60">Available Balance</div>
                      <div className="text-5xl font-bold font-mono text-white mb-6">
                        {user.credits.toLocaleString()} <span className="text-secondary text-2xl">CR</span>
                      </div>

                      <div className="flex gap-4">
                        <div className="flex-1 p-3 bg-white/5 rounded-xl border border-white/5">
                          <div className="text-[10px] text-text-muted uppercase font-bold mb-1">Lifetime Earned</div>
                          <div className="text-lg font-mono text-success">12,540 CR</div>
                        </div>
                        <div className="flex-1 p-3 bg-white/5 rounded-xl border border-white/5">
                          <div className="text-[10px] text-text-muted uppercase font-bold mb-1">Today's Income</div>
                          <div className="text-lg font-mono text-primary">+84 CR</div>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Info Card */}
                  <div className="bg-surface/30 border border-white/5 rounded-3xl p-6 flex flex-col justify-center">
                    <h3 className="font-bold text-white mb-2 flex items-center gap-2">
                      <Radio className="w-4 h-4 text-primary" /> How to earn more?
                    </h3>
                    <p className="text-sm text-text-muted leading-relaxed">
                      WorldVPN credits are earned by sharing your unused internet bandwidth.
                      The more stable and fast your connection is, the higher your reputation score and rewards.
                    </p>
                    <div className="mt-4 grid grid-cols-2 gap-3">
                      <div className="text-xs p-2 bg-primary/5 border border-primary/20 rounded-lg text-primary text-center">
                        High Speed: +20%
                      </div>
                      <div className="text-xs p-2 bg-secondary/5 border border-secondary/20 rounded-lg text-secondary text-center">
                        Stable: +15%
                      </div>
                    </div>
                  </div>
                </div>

                <div className="mt-8">
                  <h3 className="font-bold text-white mb-4 flex items-center gap-2">
                    <HistoryIcon className="w-4 h-4 text-text-muted" /> Transaction History
                  </h3>
                  <div className="space-y-2">
                    {[
                      { type: "Sharing Reward", date: "Today, 14:05", amount: "+12.00 CR", status: "success" },
                      { type: "Sharing Reward", date: "Today, 13:02", amount: "+8.50 CR", status: "success" },
                      { type: "VPN Connection", date: "Yesterday, 22:15", amount: "-10.00 CR", status: "usage" },
                      { type: "Sharing Reward", date: "Yesterday, 20:00", amount: "+42.20 CR", status: "success" },
                    ].map((tx, i) => (
                      <div key={i} className="flex justify-between items-center p-4 bg-surface/30 hover:bg-surface-highlight/40 transition-colors rounded-2xl border border-white/5">
                        <div className="flex items-center gap-4">
                          <div className={`w-10 h-10 rounded-full flex items-center justify-center ${tx.status === 'success' ? 'bg-success/10 text-success' : 'bg-primary/10 text-primary'}`}>
                            {tx.status === 'success' ? <Activity className="w-4 h-4" /> : <Shield className="w-4 h-4" />}
                          </div>
                          <div>
                            <div className="text-sm font-medium text-white">{tx.type}</div>
                            <div className="text-[10px] text-text-muted font-mono">{tx.date}</div>
                          </div>
                        </div>
                        <div className={`font-mono font-bold ${tx.status === 'success' ? 'text-success' : 'text-white/60'}`}>{tx.amount}</div>
                      </div>
                    ))}
                  </div>
                </div>
              </motion.div>
            )}

            {activeTab === 'settings' && (
              <motion.div
                key="settings"
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                className="absolute inset-0 p-8 overflow-y-auto"
              >
                <h2 className="text-2xl font-bold text-white mb-6 tracking-tight">System Configuration</h2>

                <div className="grid grid-cols-1 lg:grid-cols-3 gap-8 max-w-7xl">
                  {/* Protocol Section */}
                  <div className="lg:col-span-2 space-y-4">
                    <div className="bg-surface/30 border border-white/10 rounded-3xl p-6">
                      <div className="flex justify-between items-center mb-6">
                        <h3 className="text-sm font-bold text-white uppercase tracking-widest opacity-60">Security Protocols</h3>
                        <span className="text-[10px] text-primary bg-primary/10 px-2 py-0.5 rounded border border-primary/20 font-bold uppercase">Multi-Engine</span>
                      </div>

                      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                        <ProtocolOption
                          id="wg"
                          icon={Shield}
                          title="WireGuard"
                          desc="Highest performance, modern cryptography"
                          active={true}
                        />
                        <ProtocolOption
                          id="ss"
                          icon={Globe}
                          title="Shadowsocks"
                          desc="Optimized for performance and stealth"
                          active={false}
                        />
                        <ProtocolOption
                          id="ovpn"
                          icon={Lock}
                          title="OpenVPN (UDP/TCP)"
                          desc="Universal compatibility, firewalls bypass"
                          active={false}
                        />
                        <ProtocolOption
                          id="h2"
                          icon={Activity}
                          title="Hysteria 2"
                          desc="QUIC-based, best for unstable mobile links"
                          active={false}
                        />
                        <ProtocolOption
                          id="trojan"
                          icon={Shield}
                          title="Trojan"
                          desc="Mimics TLS traffic to bypass deep inspection"
                          active={false}
                        />
                        <ProtocolOption
                          id="vless"
                          icon={Cpu}
                          title="VLESS"
                          desc="Advanced lightweight stealth protocol"
                          active={false}
                        />
                      </div>
                    </div>

                    <div className="bg-surface/30 border border-white/10 rounded-3xl p-6">
                      <h3 className="text-sm font-bold text-white mb-4 uppercase tracking-widest opacity-60">General Settings</h3>
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-6">
                        <SettingsToggle title="Auto-Connect" desc="Start VPN automatically on system boot" active={true} />
                        <SettingsToggle title="Kill Switch" desc="Block internet if VPN connection drops" active={false} />
                        <SettingsToggle title="DNS Leak Protection" desc="Force secure DNS servers" active={true} />
                        <SettingsToggle title="IPv6 Leak Protection" desc="Disable IPv6 during active sessions" active={true} />
                        <SettingsToggle title="Local LAN" desc="Allow access to local network devices" active={true} />
                        <SettingsToggle title="Smart Routing" desc="Route only international traffic" active={false} />
                      </div>
                    </div>
                  </div>

                  {/* Account & Meta Section */}
                  <div className="space-y-4">
                    <div className="bg-gradient-to-br from-surface/50 to-surface-highlight/20 border border-white/10 rounded-3xl p-6">
                      <h3 className="text-sm font-bold text-white mb-6 uppercase tracking-widest opacity-60">Active Account</h3>
                      <div className="flex items-center gap-4 mb-6">
                        <div className="w-16 h-16 rounded-2xl bg-primary/20 border border-primary/30 flex items-center justify-center">
                          <Users className="w-8 h-8 text-primary" />
                        </div>
                        <div>
                          <div className="text-lg font-bold text-white leading-tight">alpha_user_01</div>
                          <div className="text-xs text-text-muted font-mono">ID: 8v9a-7f21-k9s1</div>
                          <div className="inline-block mt-1 px-2 py-0.5 bg-success/10 text-success text-[10px] rounded font-bold uppercase">Pro Early Bird</div>
                        </div>
                      </div>
                      <button className="w-full py-3 rounded-xl bg-white/5 border border-white/10 text-white font-medium hover:bg-white/10 hover:border-white/20 transition-all">
                        Manage Subscription
                      </button>
                    </div>

                    <div className="bg-surface/30 border border-white/10 rounded-3xl p-6 text-center">
                      <p className="text-xs text-text-muted mb-4 opacity-70">WorldVPN Desktop v0.1.0-beta</p>
                      <div className="flex flex-col gap-2">
                        <button className="w-full py-2 bg-primary/5 hover:bg-primary/10 border border-primary/20 rounded-lg text-[10px] font-bold text-primary uppercase transition-all tracking-widest">
                          Check for Updates
                        </button>
                        <div className="flex justify-center gap-4 mt-2">
                          <button className="text-[10px] text-text-muted hover:text-white transition-colors">Privacy Policy</button>
                          <button className="text-[10px] text-danger hover:underline">Logout Session</button>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </main>
    </div>
  );
}

function NavIcon({ icon: Icon, active, onClick, label }: any) {
  return (
    <div className="relative group cursor-pointer w-12 h-12 flex items-center justify-center" onClick={onClick}>
      {active && (
        <motion.div
          layoutId="activeNav"
          className="absolute inset-0 bg-primary/10 rounded-xl border border-primary/30 shadow-[0_0_15px_rgba(0,242,234,0.1)]"
        />
      )}
      <Icon className={`w-6 h-6 z-10 transition-colors ${active ? 'text-primary' : 'text-text-muted group-hover:text-white'}`} />

      {/* Tooltip */}
      <div className="absolute left-14 bg-surface border border-white/10 px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity text-xs font-medium pointer-events-none whitespace-nowrap z-50">
        {label}
      </div>
    </div>
  );
}

function StatCard({ icon: Icon, label, value, sub, color }: any) {
  return (
    <div className="bg-surface/30 backdrop-blur-sm border border-white/5 rounded-2xl p-4 flex flex-col justify-between hover:bg-surface/50 transition-colors">
      <div className="flex justify-between items-start">
        <span className="text-text-muted text-xs font-bold uppercase tracking-wider">{label}</span>
        <Icon className={`w-4 h-4 ${color} opacity-80`} />
      </div>
      <div>
        <div className="text-lg font-bold text-white">{value}</div>
        <div className={`text-[10px] ${color} opacity-80 font-mono mt-0.5`}>{sub}</div>
      </div>
    </div>
  )
}

function MapNode({ top, left, label, latency, active }: any) {
  return (
    <motion.div
      initial={{ scale: 0, opacity: 0 }}
      animate={{ scale: 1, opacity: 1 }}
      className="absolute group/node cursor-pointer z-20"
      style={{ top, left }}
    >
      <div className={`relative ${active ? 'z-30' : 'z-20'}`}>
        <div className={`w-3 h-3 rounded-full ${active ? 'bg-primary' : 'bg-secondary'} relative z-10 shadow-[0_0_10px_currentColor]`}>
          {active && <div className="absolute inset-0 rounded-full bg-primary animate-ping opacity-75" />}
        </div>

        {/* Tooltip */}
        <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 opacity-0 group-hover/node:opacity-100 transition-opacity pointer-events-none">
          <div className="bg-surface/90 backdrop-blur-md border border-white/10 px-3 py-1.5 rounded-lg shadow-2xl whitespace-nowrap">
            <div className="text-[10px] font-bold text-white mb-0.5">{label}</div>
            <div className="text-[9px] text-text-muted flex items-center gap-1">
              <Activity className="w-2 h-2" /> {latency} throughput
            </div>
          </div>
          <div className="w-2 h-2 bg-surface/90 border-r border-b border-white/10 rotate-45 absolute -bottom-1 left-1/2 -translate-x-1/2" />
        </div>
      </div>
    </motion.div>
  );
}

function ProtocolOption({ icon: Icon, title, desc, active }: any) {
  return (
    <label className={`flex items-center justify-between p-3 rounded-xl border cursor-pointer transition-all ${active ? 'bg-primary/5 border-primary/30' : 'hover:bg-white/5 border-transparent'}`}>
      <div className="flex items-center gap-3">
        <Icon className={`w-5 h-5 ${active ? 'text-primary' : 'text-text-muted'}`} />
        <div>
          <div className={`font-medium ${active ? 'text-white' : 'text-white/60'}`}>{title}</div>
          <div className="text-[10px] text-text-muted">{desc}</div>
        </div>
      </div>
      <div className={`w-4 h-4 rounded-full border-2 transition-all ${active ? 'border-primary bg-primary shadow-[0_0_10px_rgba(0,242,234,0.5)]' : 'border-text-muted'}`} />
    </label>
  );
}

function SettingsToggle({ title, desc, active }: any) {
  return (
    <div className="flex items-center justify-between group">
      <div>
        <div className="text-sm font-medium text-white group-hover:text-primary transition-colors">{title}</div>
        <div className="text-[10px] text-text-muted">{desc}</div>
      </div>
      <div className={`w-10 h-5 rounded-full relative cursor-pointer transition-colors ${active ? 'bg-primary/20 border border-primary/30' : 'bg-surface-highlight border border-white/10'}`}>
        <motion.div
          animate={{ x: active ? 22 : 2 }}
          className={`absolute top-0.5 w-3.5 h-3.5 rounded-full ${active ? 'bg-primary shadow-[0_0_10px_rgba(0,242,234,0.5)]' : 'bg-text-muted'}`}
        />
      </div>
    </div>
  );
}

export default App;
