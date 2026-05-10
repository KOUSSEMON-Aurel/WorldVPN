import { useState, useEffect, useRef } from "react";
import { Shield, Globe, Wallet, Settings, Power, Activity, Users, Radio, LogOut, Zap } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { LeafletMap } from "./LeafletMap";
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
  provider?: string;
  protocol?: string;
  ovpn_config?: string;
  ss_metadata?: string;
}


// Mock Data
interface User {
  username: string;
  credits: number;
  token: string;
  publicKey?: string;
}

function App() {
  const [user, setUser] = useState<User | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("home");
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");
  const [isSharing, setIsSharing] = useState(true);
  const [nodes, setNodes] = useState<Node[]>([]);
  const [nodeGroup, setNodeGroup] = useState<NodeGroup>("COMMUNITY");
  const [traffic, setTraffic] = useState({ down: 0, up: 0 });
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [importKeyVal, setImportKeyVal] = useState("");
  const [importError, setImportError] = useState("");
  const [initError, setInitError] = useState<string | null>(null);
  const [selectedProtocols, setSelectedProtocols] = useState<string[]>(["OpenVPN", "Shadowsocks"]);

  const AVAILABLE_PROTOCOLS = ["OpenVPN", "Shadowsocks", "L2TP", "SSTP"];

  // removed P2P Stats poll as we are not using the mock

  // Load basic settings
  useEffect(() => {
    const savedGroup = localStorage.getItem("worldvpn_nodeGroup");
    if (savedGroup === "COMMUNITY" || savedGroup === "PUBLIC") setNodeGroup(savedGroup as NodeGroup);

    const savedSharing = localStorage.getItem("worldvpn_isSharing");
    if (savedSharing !== null) setIsSharing(savedSharing === "true");
  }, []);

  // Save basic settings
  useEffect(() => {
    localStorage.setItem("worldvpn_nodeGroup", nodeGroup);
  }, [nodeGroup]);

  useEffect(() => {
    localStorage.setItem("worldvpn_isSharing", isSharing.toString());
  }, [isSharing]);

  // Auto-login or Register if no key
  const initRun = useRef(false);
  useEffect(() => {
    if (initRun.current) return;
    initRun.current = true;

    const initIdentity = async () => {
      console.debug("initIdentity started");
      setInitError(null);
      try {
        const isSaved: boolean = await invoke("is_identity_saved");
        console.debug("Saved key found:", isSaved);
        if (isSaved) {
          const success = await login();
          if (!success) {
            console.warn("Auto-login failed with saved key, attempting to register fresh identity...");
            await handleRegister();
          }
        } else {
          await handleRegister();
        }
      } catch (e: any) {
        console.error("Initialization failed", e);
        const errorMsg = typeof e === 'string' ? e : (e.message || JSON.stringify(e));
        setInitError(`Initialization failed: ${errorMsg}`);
      }
    };
    initIdentity();
  }, []);

  const [actualIp, setActualIp] = useState("Checking...");
  const [connectedCountry, setConnectedCountry] = useState<string | null>(null);
  const [latency, setLatency] = useState<number | null>(null);

  // Deriving filtered nodes array
  const filteredNodes = nodes.filter(n => {
    if (selectedProtocols.length === 0) return true;
    return selectedProtocols.includes(n.protocol || "");
  });

  // Poll VPN Metrics & Status
  useEffect(() => {
    if (status !== 'connected') {
      setTraffic({ down: 0, up: 0 });
      setActualIp("192.168.1.42");
      setLatency(null);
      return;
    }
    const interval = setInterval(async () => {
      try {
        const metrics: any = await invoke("get_vpn_metrics");
        setTraffic({ down: metrics.down_mbps, up: metrics.up_mbps });
        setLatency(metrics.latency_ms);

        const vpnStatus: any = await invoke("get_vpn_status");
        if (vpnStatus.state === "Connected") {
          if (vpnStatus.current_ip) setActualIp(vpnStatus.current_ip);
          if (vpnStatus.country) setConnectedCountry(vpnStatus.country);
        } else {
          setConnectedCountry(null);
        }
      } catch (e) {
        console.error("Metrics polling failed", e);
      }
    }, 2000);
    return () => clearInterval(interval);
  }, [status]);

  const login = async (): Promise<boolean> => {
    try {
      console.debug("Invoking login_anonymously_desktop");
      const response: any = await invoke("login_anonymously_desktop");
      console.debug("Login response received:", response);
      setUser({
        username: response.username,
        credits: 0,
        token: response.token,
        publicKey: response.username,
      });
      return true;
    } catch (e: any) {
      console.error("Login failed inner context:", e);
      return false;
    }
  };

  // Poll Wallet Balance
  useEffect(() => {
    const updateBalance = async () => {
      if (!user?.token) return;
      try {
        const balance: any = await invoke("get_wallet_balance_desktop", { token: user.token });
        setUser(prev => prev ? { ...prev, credits: balance } : null);
      } catch (e) {
        console.error("Failed to fetch balance", e);
      }
    };
    updateBalance();
    const int = setInterval(updateBalance, 10000);
    return () => clearInterval(int);
  }, [user?.token, activeTab]);

  const handleRegister = async () => {
    try {
      console.debug("Invoking generate_identity...");
      const identity: any = await invoke("generate_identity");
      console.debug("New identity generated", identity);
      const success = await login();
      if (!success) {
        throw new Error("Automatic login failed after registration.");
      }
    } catch (e: any) {
      console.error("Failed to generate identity", e);
      throw e;
    }
  };

  const handleLogout = () => {
    setUser(null);
  };

  const handleImportKeySubmit = async () => {
    try {
      const arr = JSON.parse(importKeyVal);
      if (!Array.isArray(arr) || arr.length === 0) {
        setImportError("Invalid format: Must be a JSON array of numbers");
        return;
      }
      if (arr.some(n => typeof n !== 'number' || n < 0 || n > 255)) {
        setImportError("Invalid format: Array elements must be valid byte values (0-255)");
        return;
      }

      const response: any = await invoke("import_identity", { privateKey: arr });
      setUser({
        username: response.username,
        credits: 0,
        token: response.token,
        publicKey: response.username,
      });
      setShowImportDialog(false);
      setImportKeyVal("");
      setImportError("");
    } catch (e: any) {
      setImportError(`Failed to import key: ${e.message || e}`);
    }
  };

  // Fetch nodes from backend
  useEffect(() => {
    const fetchNodes = async () => {
      try {
        const backendNodes: any = await invoke("get_nodes", { group: nodeGroup });
        if (Array.isArray(backendNodes)) {
          setNodes(backendNodes);
        }
      } catch (e) {
        console.error("Failed to fetch nodes", e);
      }
    };
    fetchNodes();
    const interval = setInterval(fetchNodes, 10000);
    return () => clearInterval(interval);
  }, [nodeGroup]);
  const toggleConnection = async () => {
    if (status === "disconnected") {
      setStatus("connecting");
      try {
        await invoke("connect_vpn", {
          protocol: "WireGuard",
          country: nodeGroup === "COMMUNITY" ? "FR" : "US",
          token: user?.token || "",
          ovpnConfig: null,
          ssMetadata: null
        });
        setStatus("connected");
      } catch (e: any) {
        console.error("Connection failed", e);
        setStatus("disconnected");
        alert("Failed to connect: " + (e.message || e));
      }
    } else {
      try {
        await invoke("disconnect_vpn");
      } catch (e: any) {
        console.error("Disconnect failed", e);
      }
      setStatus("disconnected");
    }
  };

  const connectToNode = async (node: Node) => {
    setActiveTab("home");
    setNodeGroup(node.group as NodeGroup);

    if (status === "connected") {
      try {
        await invoke("disconnect_vpn");
      } catch (e) {
        console.warn("Disconnect failed or already disconnected", e);
      }
    }

    setStatus("connecting");
    try {
      await invoke("connect_vpn", {
        protocol: node.protocol || "WireGuard",
        country: node.country_code,
        token: user?.token || "",
        ovpnConfig: node.ovpn_config || null,
        ssMetadata: node.ss_metadata || null
      });
      setStatus("connected");
    } catch (e: any) {
      console.error("Connection failed", e);
      setStatus("disconnected");
      alert("Failed to connect: " + (e.message || e));
    }
  };

  if (!user) {
    return (
      <div className="flex h-screen w-screen bg-background items-center justify-center p-6">
        <div className="flex flex-col items-center gap-4 max-w-sm text-center">
          <Shield className={`w-12 h-12 ${initError ? 'text-danger' : 'text-primary animate-pulse'}`} />
          <h1 className="text-xl font-bold text-white">
            {initError ? "Initialization Failed" : "Initializing WorldVPN..."}
          </h1>
          <p className="text-text-muted text-sm px-4">
            {initError || "Synchronizing Secure Identity"}
          </p>

          {initError && (
            <div className="flex gap-3 mt-4">
              <button
                onClick={() => window.location.reload()}
                className="px-4 py-2 bg-surface-highlight hover:bg-white/10 rounded-lg text-xs font-bold transition-all"
              >
                RETRY
              </button>
              <button
                onClick={() => {
                  window.location.reload();
                }}
                className="px-4 py-2 border border-danger/30 text-danger hover:bg-danger/10 rounded-lg text-xs font-bold transition-all"
              >
                RESET IDENTITY
              </button>
            </div>
          )}
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
            <p className="text-xs text-text-muted font-mono mt-1 opacity-70">IP: {status === 'connected' ? `${actualIp} (Protected)` : 'Detecting... (Exposed)'}</p>
          </div>

          <div className="flex items-center gap-4">
            <div className="bg-surface-highlight/50 border border-white/5 px-4 py-2 rounded-lg flex items-center gap-3">
              <div className="bg-secondary/20 p-1.5 rounded-md">
                <Wallet className="w-4 h-4 text-secondary" />
              </div>
              <div>
                <div className="text-sm font-mono font-bold text-white leading-none">{user?.credits?.toLocaleString() || "0"} CR</div>
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
                                <Shield className="w-4 h-4" /> {connectedCountry || "Secure Tunnel"}
                              </span>
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-3 gap-4 h-32">
                    <StatCard icon={Activity} label="Latency" value={status === 'connected' ? (latency ? `${latency} ms` : "--") : "--"} sub="Optimized" color="text-success" />
                    <StatCard icon={Zap} label="Download" value={status === 'connected' ? `${traffic.down?.toFixed(1) || 0} Mbps` : "--"} sub="Global Route" color="text-primary" />
                    <StatCard icon={Zap} label="Upload" value={status === 'connected' ? `${traffic.up?.toFixed(1) || 0} Mbps` : "--"} sub="Secure Tunnel" color="text-secondary" />
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
                  <div className="p-4 border-b border-white/5 bg-white/[0.02] flex flex-wrap gap-2">
                    {AVAILABLE_PROTOCOLS.map(proto => {
                      const isActive = selectedProtocols.includes(proto);
                      return (
                        <button
                          key={proto}
                          onClick={() => {
                            if (isActive) {
                              setSelectedProtocols(selectedProtocols.filter(p => p !== proto));
                            } else {
                              setSelectedProtocols([...selectedProtocols, proto]);
                            }
                          }}
                          className={`px-3 py-1 rounded-full text-[10px] font-bold uppercase tracking-wider transition-all border ${isActive
                            ? "bg-primary/20 text-primary border-primary/50"
                            : "bg-surface text-text-muted border-white/10 hover:border-white/30"
                            }`}
                        >
                          {proto}
                        </button>
                      );
                    })}
                  </div>
                  <div className="flex-1 overflow-y-auto p-4 space-y-3">
                    <div className="flex justify-between items-center mb-2 px-2">
                      <span className="text-xs text-text-muted font-bold uppercase">{filteredNodes.length} Servers Found</span>
                    </div>
                    {filteredNodes.slice(0, 50).map((node: Node, i: number) => (
                      <motion.div key={node.id} initial={{ opacity: 0, x: 20 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: Math.min(i * 0.05, 0.5) }} className="bg-surface-highlight/50 rounded-xl p-3 border border-white/5 flex items-center justify-between hover:border-white/20 transition-all cursor-pointer" onClick={() => connectToNode(node)}>
                        <div className="flex items-center gap-3">
                          <div className="w-10 h-10 rounded-lg bg-surface flex items-center justify-center font-bold text-sm border border-white/5 shadow-sm text-white/90">{node.country_code}</div>
                          <div className="flex flex-col">
                            <div className="text-sm font-medium text-white flex items-center gap-2">
                              {node.provider} <span className="text-[9px] px-1.5 py-0.5 rounded bg-white/5 text-primary border border-primary/20">{node.protocol}</span>
                            </div>
                            <div className="text-[10px] text-text-muted flex items-center gap-2 mt-0.5">
                              <span className="text-success font-mono font-bold">{node.latency_ms}ms</span>
                              <span className="opacity-50">|</span>
                              <span className="text-secondary font-mono font-bold">{node.bandwidth_mbps} Mbps</span>
                            </div>
                          </div>
                        </div>
                        <div className="text-right">
                          <button className="px-3 py-1.5 rounded-lg text-xs font-bold bg-white/5 hover:bg-primary/20 hover:text-primary transition-all text-text-muted">Connect</button>
                        </div>
                      </motion.div>
                    ))}
                    {filteredNodes.length === 0 && (
                      <div className="h-40 flex flex-col items-center justify-center opacity-40">
                        <span className="text-sm font-bold mt-2">No nodes matches your filter</span>
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
                  <LeafletMap nodes={filteredNodes} onConnect={connectToNode} nodeGroup={nodeGroup} />
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
                <div className="flex flex-col gap-6">
                  <div className="bg-surface/30 border border-white/10 rounded-3xl p-6">
                    <div className="flex items-center gap-4 mb-6">
                      <div className="w-16 h-16 rounded-2xl bg-primary/20 flex items-center justify-center">
                        <Users className="w-8 h-8 text-primary" />
                      </div>
                      <div className="flex-1">
                        <div className="text-lg font-bold text-white">{user.username}</div>
                        <div className="text-xs text-text-muted font-mono truncate max-w-[300px]">{user.publicKey || 'Anonymous'}</div>
                      </div>
                    </div>

                    <div className="space-y-4">
                      <div>
                        <label className="text-[10px] font-bold text-text-muted uppercase mb-2 block">Secure Identity Key</label>
                        <div className="flex gap-2">
                          <input
                            type="text"
                            readOnly
                            value={"[SECURE STORAGE IN RUST] - " + (user.publicKey || "")}
                            className="flex-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-xs font-mono text-white outline-none opacity-50"
                          />
                        </div>
                      </div>

                      <div className="pt-4 border-t border-white/5 flex flex-wrap gap-3">
                        <button
                          onClick={() => setShowImportDialog(true)}
                          className="text-primary text-xs font-bold hover:underline"
                        >
                          Restore Identity
                        </button>
                        <button
                          onClick={async () => {
                            if (confirm("Are you sure? This will generate a new identity and you will lose access to your current account credits unless you have backed up your key.")) {
                              await handleRegister();
                            }
                          }}
                          className="text-secondary text-xs font-bold hover:underline"
                        >
                          Regenerate Identity
                        </button>
                        <button
                          onClick={handleLogout}
                          className="text-danger text-xs font-bold ml-auto"
                        >
                          Clear Session
                        </button>
                      </div>
                    </div>
                  </div>

                  <div className="bg-surface/30 border border-white/10 rounded-3xl p-6">
                    <h3 className="text-sm font-bold text-white mb-4">About WorldVPN</h3>
                    <div className="mt-8 text-[10px] text-text-muted font-mono space-y-1">
                      <p className="flex items-center gap-2"><span>•</span> Version: 1.0.0-beta.1 | Protocol: Unified sing-box (Go) | Identity: Ed25519 Elliptic Curve</p>
                      <p className="flex items-center gap-2 text-primary/50"><span>•</span> Status: Initialized • {user ? "Authenticated" : "Anonymous"}</p>
                    </div>
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </main>

      {/* Import Import Modal */}
      <AnimatePresence>
        {showImportDialog && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="absolute inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
            <div className="bg-surface border border-white/10 p-6 rounded-2xl w-[400px]">
              <h3 className="text-lg font-bold text-white mb-4">Import Identity</h3>
              <p className="text-xs text-text-muted mb-4">Paste your identity key (JSON array of bytes) below.</p>
              <textarea
                className="w-full h-32 bg-white/5 border border-white/10 rounded-lg p-3 text-xs font-mono text-white outline-none focus:border-primary/50 transition-colors resize-none mb-2"
                value={importKeyVal}
                onChange={(e) => setImportKeyVal(e.target.value)}
                placeholder="[123, 45, ...]"
              />
              {importError && <p className="text-danger text-xs mb-4 font-bold">{importError}</p>}
              <div className="flex gap-3 justify-end items-center mt-2">
                <button
                  onClick={() => { setShowImportDialog(false); setImportError(""); }}
                  className="px-4 py-2 hover:bg-white/5 rounded-lg text-xs font-bold transition-all text-text-muted"
                >
                  CANCEL
                </button>
                <button
                  onClick={handleImportKeySubmit}
                  className="px-4 py-2 bg-primary text-black rounded-lg text-xs font-bold transition-all hover:scale-105"
                >
                  IMPORT
                </button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function NavIcon({ icon: Icon, active, onClick, label }: { icon: any, active?: boolean, onClick?: () => void, label?: string }) {
  return (
    <motion.button title={label} whileHover={{ scale: 1.1 }} whileTap={{ scale: 0.9 }} onClick={onClick} className={`p-3 rounded-xl transition-all duration-300 relative group ${active ? 'bg-primary/20 text-primary' : 'text-text-muted hover:bg-surface-highlight hover:text-white'}`}>
      <Icon className="w-5 h-5" />
    </motion.button>
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

export default App;
