import { useState, useEffect } from "react";
import { Shield, Globe, Wallet, Settings, Power, Activity, Lock, Users, Radio, Cpu, LogOut, History as HistoryIcon, Zap } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
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
  publicKey?: string;
}

function App() {
  const [user, setUser] = useState<User | null>(null);
  const [authMode, setAuthMode] = useState<"login" | "register">("login");
  const [privateKeyInput, setPrivateKeyInput] = useState("");
  const [activeTab, setActiveTab] = useState<Tab>("home");
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");
  const [isSharing, setIsSharing] = useState(true);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [nodeGroup, setNodeGroup] = useState<NodeGroup>("COMMUNITY");
  const [traffic, setTraffic] = useState({ down: 0, up: 0 });
  const [p2pStats, setP2pStats] = useState({ connected_peers: 0, known_nodes: 0, total_sent: 0, total_received: 0 });
  const [error, setError] = useState<string | null>(null);

  // Poll P2P Stats
  useEffect(() => {
    if (!isSharing) return;
    const interval = setInterval(async () => {
      try {
        const stats: any = await invoke("get_p2p_status");
        setP2pStats(stats);
      } catch (e) {
        console.error("P2P polling failed", e);
      }
    }, 5000);
    return () => clearInterval(interval);
  }, [isSharing]);

  // Auto-login if private key is in localStorage
  useEffect(() => {
    const savedKey = localStorage.getItem("worldvpn_private_key");
    if (savedKey) {
      try {
        const keyArray = JSON.parse(savedKey);
        loginWithKey(keyArray);
      } catch (e) {
        console.error("Failed to auto-login", e);
      }
    }
  }, []);

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

  const loginWithKey = async (keyArray: number[]) => {
    try {
      const response: any = await invoke("login_anonymously_desktop", { privateKey: keyArray });
      setUser({
        username: response.username,
        credits: 50,
        token: response.token,
        publicKey: response.username,
      });
      localStorage.setItem("worldvpn_private_key", JSON.stringify(keyArray));
    } catch (e: any) {
      setError(e.toString());
    }
  };

  const handleRegister = async () => {
    try {
      const identity: any = await invoke("generate_identity");
      await loginWithKey(identity.private_key);
    } catch (e: any) {
      setError("Failed to generate identity: " + e.toString());
    }
  };

  const handleAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (authMode === 'register') {
      await handleRegister();
    } else {
      try {
        const keyArray = JSON.parse(privateKeyInput);
        await loginWithKey(keyArray);
      } catch (e) {
        setError("Invalid Access Key format (must be JSON array of bytes)");
      }
    }
  };

  const handleGuestLogin = () => {
    setUser({
      username: "Guest",
      credits: 0,
      token: "guest-token"
    });
    setNodeGroup("PUBLIC");
  };

  const handleLogout = () => {
    setUser(null);
    localStorage.removeItem("worldvpn_private_key");
    setPrivateKeyInput("");
  };

  // Fetch nodes from backend
  useEffect(() => {
    const fetchNodes = async () => {
      try {
        if (activeTab === 'map') {
          // In a real app we'd use invoke("discover_nodes")
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
            <p className="text-text-muted text-sm mt-2">P2P Anonymous Identity</p>
          </div>

          <form onSubmit={handleAuth} className="space-y-4">
            {error && (
              <div className="p-3 bg-danger/10 border border-danger/20 rounded-xl text-danger text-xs font-medium">
                {error}
              </div>
            )}

            {authMode === 'login' && (
              <div className="space-y-1">
                <label className="text-[10px] font-bold text-text-muted uppercase ml-1">Access Key (JSON Byte Array)</label>
                <textarea
                  value={privateKeyInput}
                  onChange={(e) => setPrivateKeyInput(e.target.value)}
                  className="w-full h-24 bg-white/5 border border-white/10 rounded-xl px-4 py-3 text-white focus:border-primary/50 outline-none transition-all font-mono text-[10px]"
                  placeholder="[1, 2, 3, ...]"
                  required
                />
              </div>
            )}

            {authMode === 'register' && (
              <div className="p-4 bg-primary/5 border border-primary/10 rounded-2xl text-center mb-4">
                <p className="text-xs text-text-muted">Registering will generate a unique Ed25519 identity key. No email or password required.</p>
              </div>
            )}

            <button type="submit" className="w-full bg-primary text-background font-bold py-4 rounded-xl mt-4 hover:scale-[1.02] active:scale-[0.98] transition-all shadow-[0_0_20px_rgba(0,242,234,0.3)]">
              {authMode === 'login' ? 'RESTORE IDENTITY' : 'GENERATE NEW IDENTITY'}
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

          <div className="mt-4 flex justify-center text-xs">
            <button
              onClick={handleGuestLogin}
              className="text-text-muted hover:text-white transition-colors"
            >
              Access as Guest
            </button>
          </div>
        </motion.div>

        <div className="absolute top-10 left-10 opacity-20 font-mono text-[10px] text-primary">
          IDENTITY_MODE: ED25519<br />P2P_NETWORK: CONNECTING<br />ZERO_LOG: ACTIVE
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen w-screen text-text-main overflow-hidden bg-background font-sans">
      <div className="ambient-glow opacity-30" />

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
            onClick={async () => {
              const newMode = !isSharing;
              setIsSharing(newMode);
              if (newMode) await invoke("start_sharing");
              else await invoke("stop_sharing");
            }}
            title={isSharing ? "Sharing Enabled" : "Sharing Paused"}
          >
            <Radio className={`w-5 h-5 ${isSharing ? 'animate-pulse' : ''}`} />
          </div>

          <div className="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer text-danger hover:bg-danger/10 transition-all" onClick={handleLogout}>
            <LogOut className="w-5 h-5" />
          </div>
        </div>
      </nav>

      <main className="flex-1 flex flex-col relative z-10">
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

        <div className="flex-1 overflow-hidden relative">
          <AnimatePresence mode="wait">
            {activeTab === 'home' && (
              <motion.div
                key="home"
                initial={{ opacity: 0, scale: 0.98 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.98 }}
                className="absolute inset-0 p-4 md:p-8 grid grid-cols-1 md:grid-cols-12 gap-6 overflow-y-auto md:overflow-hidden"
              >
                <div className="md:col-span-8 flex flex-col gap-6">
                  <div className="h-[300px] md:flex-1 relative rounded-3xl overflow-hidden border border-white/5 bg-surface/40 flex items-center justify-center group">
                    <div className="absolute inset-0 opacity-10 bg-[url('https://upload.wikimedia.org/wikipedia/commons/e/ec/World_map_blank_without_borders.svg')] bg-cover bg-center mix-blend-overlay transition-opacity duration-1000 group-hover:opacity-20" />
                    {status === 'connecting' && (
                      <div className="absolute inset-0 flex items-center justify-center">
                        <div className="w-[500px] h-[500px] border border-primary/20 rounded-full animate-ping opacity-20" />
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
                            <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -10 }} className="flex flex-col items-center">
                              <span className="text-secondary font-bold text-lg flex items-center gap-2">
                                <Shield className="w-4 h-4" /> Tokyo, Japan
                              </span>
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-3 gap-4 h-32">
                    <StatCard icon={Activity} label="Latency" value={status === 'connected' ? "15 ms" : "--"} sub="Optimized" color="text-success" />
                    <StatCard icon={Zap} label="Download" value={status === 'connected' ? `${traffic.down.toFixed(1)} Mbps` : "--"} sub="Global Route" color="text-primary" />
                    <StatCard icon={Zap} label="Upload" value={status === 'connected' ? `${traffic.up.toFixed(1)} Mbps` : "--"} sub="Secure Tunnel" color="text-secondary" />
                  </div>
                </div>

                <div className="col-span-4 bg-surface/30 backdrop-blur-md rounded-3xl border border-white/5 flex flex-col overflow-hidden">
                  <div className="p-6 border-b border-white/5 flex justify-between items-center bg-white/[0.02]">
                    <h2 className="font-bold flex items-center gap-2 text-sm uppercase tracking-wider">
                      <Activity className="w-4 h-4 text-secondary" />
                      Live Nodes
                    </h2>
                    <div className={`w-2 h-2 rounded-full ${isSharing ? 'bg-success shadow-[0_0_10px_#00ff9d]' : 'bg-red-500'}`} />
                  </div>
                  <div className="flex-1 overflow-y-auto p-4 space-y-3">
                    {isSharing ? (
                      <>
                        <div className="bg-primary/5 border border-primary/20 rounded-2xl p-4 mb-4">
                          <div className="flex justify-between items-center mb-3">
                            <span className="text-[10px] font-bold text-primary uppercase">P2P Network Active</span>
                            <Radio className="w-3 h-3 text-primary animate-pulse" />
                          </div>
                          <div className="grid grid-cols-2 gap-4">
                            <div>
                              <div className="text-xl font-bold text-white">{p2pStats.connected_peers}</div>
                              <div className="text-[9px] text-text-muted uppercase">Connected Peers</div>
                            </div>
                            <div>
                              <div className="text-xl font-bold text-white">{p2pStats.known_nodes}</div>
                              <div className="text-[9px] text-text-muted uppercase">Known Nodes</div>
                            </div>
                          </div>
                        </div>

                        {MOCK_SESSIONS.map((session, i) => (
                          <motion.div key={session.id} initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: i * 0.1 }} className="bg-surface-highlight/50 rounded-xl p-3 border border-white/5 flex items-center justify-between">
                            <div className="flex items-center gap-3">
                              <div className="w-10 h-10 rounded-lg bg-surface flex items-center justify-center font-bold text-sm border border-white/5">{session.country}</div>
                              <div>
                                <div className="text-sm font-medium text-white">{session.bytes}</div>
                                <div className="text-[10px] uppercase tracking-wide text-text-muted">{session.type}</div>
                              </div>
                            </div>
                            <div className="text-right">
                              <div className="text-secondary font-mono text-xs font-bold">{session.earning}</div>
                            </div>
                          </motion.div>
                        ))}
                      </>
                    ) : (
                      <div className="h-full flex flex-col items-center justify-center opacity-40 p-8">
                        <Lock className="w-12 h-12 mb-4 text-text-muted" />
                        <p className="text-sm font-bold">Sharing Disabled</p>
                      </div>
                    )}
                  </div>
                </div>
              </motion.div>
            )}

            {activeTab === 'map' && (
              <motion.div key="map" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} className="absolute inset-0 p-8 flex flex-col">
                <div className="flex justify-between items-end mb-6">
                  <div>
                    <h2 className="text-2xl font-bold text-white tracking-tight">Global Infrastructure</h2>
                    <p className="text-sm text-text-muted">Status: <span className={nodeGroup === 'COMMUNITY' ? 'text-primary' : 'text-secondary'}>{nodeGroup}</span></p>
                  </div>
                  <div className="flex bg-surface-highlight/40 p-1 rounded-xl border border-white/5">
                    <button onClick={() => setNodeGroup('COMMUNITY')} className={`px-4 py-1.5 rounded-lg text-xs font-bold ${nodeGroup === 'COMMUNITY' ? 'bg-primary' : ''}`}>Community</button>
                    <button onClick={() => setNodeGroup('PUBLIC')} className={`px-4 py-1.5 rounded-lg text-xs font-bold ${nodeGroup === 'PUBLIC' ? 'bg-secondary' : ''}`}>Public Gate</button>
                  </div>
                </div>
                <div className="flex-1 bg-surface/30 border border-white/5 rounded-3xl relative overflow-hidden flex items-center justify-center">
                  <img src="https://upload.wikimedia.org/wikipedia/commons/e/ec/World_map_blank_without_borders.svg" className="w-full max-w-4xl opacity-40 invert grayscale" />
                  {nodes.map((node) => {
                    const coords = COUNTRY_COORDS[node.country_code] || { top: "50%", left: "50%" };
                    return <MapNode key={node.id} top={coords.top} left={coords.left} label={`${node.country_code} Server`} latency={`${node.latency_ms}ms`} active={nodeGroup === 'COMMUNITY'} />;
                  })}
                </div>
              </motion.div>
            )}

            {activeTab === 'wallet' && (
              <motion.div key="wallet" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} className="absolute inset-0 p-8 overflow-y-auto">
                <div className="flex justify-between items-center mb-6">
                  <h2 className="text-2xl font-bold text-white tracking-tight">Wallet & Rewards</h2>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div className="bg-gradient-to-br from-surface/80 to-surface-highlight/40 border border-white/10 rounded-3xl p-8">
                    <div className="text-text-muted text-xs font-bold uppercase mb-1">Available Balance</div>
                    <div className="text-5xl font-bold font-mono text-white">{user.credits.toLocaleString()} CR</div>
                  </div>
                </div>
              </motion.div>
            )}

            {activeTab === 'settings' && (
              <motion.div key="settings" initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} className="absolute inset-0 p-8 overflow-y-auto">
                <h2 className="text-2xl font-bold text-white mb-6">Settings</h2>
                <div className="bg-surface/30 border border-white/10 rounded-3xl p-6">
                  <div className="flex items-center gap-4 mb-6">
                    <div className="w-16 h-16 rounded-2xl bg-primary/20 flex items-center justify-center">
                      <Users className="w-8 h-8 text-primary" />
                    </div>
                    <div>
                      <div className="text-lg font-bold text-white">{user.username}</div>
                      <div className="text-xs text-text-muted font-mono">{user.publicKey || 'Anonymous'}</div>
                    </div>
                  </div>
                  <button onClick={handleLogout} className="text-danger font-bold hover:underline">Logout Session</button>
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
      {active && <motion.div layoutId="activeNav" className="absolute inset-0 bg-primary/10 rounded-xl border border-primary/30" />}
      <Icon className={`w-6 h-6 z-10 ${active ? 'text-primary' : 'text-text-muted'}`} />
    </div>
  );
}

function StatCard({ icon: Icon, label, value, sub, color }: any) {
  return (
    <div className="bg-surface/30 border border-white/5 rounded-2xl p-4 flex flex-col justify-between">
      <div className="flex justify-between items-start">
        <span className="text-text-muted text-xs font-bold uppercase">{label}</span>
        <Icon className={`w-4 h-4 ${color}`} />
      </div>
      <div>
        <div className="text-lg font-bold text-white">{value}</div>
        <div className={`text-[10px] ${color} font-mono`}>{sub}</div>
      </div>
    </div>
  );
}

function MapNode({ top, left, label, latency, active }: any) {
  return (
    <motion.div initial={{ scale: 0, opacity: 0 }} animate={{ scale: 1, opacity: 1 }} className="absolute z-20" style={{ top, left }}>
      <div className={`w-3 h-3 rounded-full ${active ? 'bg-primary' : 'bg-secondary'}`} title={`${label} (${latency})`} />
    </motion.div>
  );
}

export default App;
