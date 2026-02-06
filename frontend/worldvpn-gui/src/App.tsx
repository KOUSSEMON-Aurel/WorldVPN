import { useState } from "react";
import "./App.css";

interface LoginState {
  username: string;
  password: string;
}

function App() {
  const [loginState, setLoginState] = useState<LoginState>({
    username: "",
    password: "",
  });
  const [token, setToken] = useState<string | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [message, setMessage] = useState("");

  const handleLogin = async () => {
    try {
      const response = await fetch("http://127.0.0.1:3000/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(loginState),
      });

      if (!response.ok) {
        const error = await response.json();
        setMessage(`❌ Erreur: ${error.error || "Login échoué"}`);
        return;
      }

      const data = await response.json();
      setToken(data.token);
      setMessage(`✅ Authentifié ! Bienvenue ${data.username}`);
    } catch (error) {
      setMessage(`❌ Erreur connexion: ${error}`);
    }
  };

  const handleConnect = async () => {
    if (!token) {
      setMessage("⚠️ Veuillez vous connecter d'abord");
      return;
    }

    try {
      const response = await fetch("http://127.0.0.1:3000/vpn/connect", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          protocol: "WireGuard",
          username: loginState.username,
        }),
      });

      if (!response.ok) {
        const error = await response.json();
        setMessage(`❌ Erreur: ${error.error || "Connexion échouée"}`);
        return;
      }

      const data = await response.json();
      setIsConnected(true);
      setMessage(
        `🔒 VPN CONNECTÉ !\\n🎯 Endpoint: ${data.server_endpoint}\\n💻 IP: ${data.assigned_ip}`
      );
    } catch (error) {
      setMessage(`❌ Erreur VPN: ${error}`);
    }
  };

  const handleDisconnect = () => {
    setIsConnected(false);
    setMessage("🔓 VPN déconnecté");
  };

  return (
    <div className="container">
      <div className="header">
        <h1>🌍 WorldVPN</h1>
        <p className="tagline">
          VPN P2P Décentralisé • Argon2 • JWT • PostgreSQL
        </p>
      </div>

      {!token ? (
        <div className="auth-section">
          <h2>🔐 Authentification</h2>
          <input
            type="text"
            placeholder="Nom d'utilisateur"
            value={loginState.username}
            onChange={(e) =>
              setLoginState({ ...loginState, username: e.target.value })
            }
          />
          <input
            type="password"
            placeholder="Mot de passe"
            value={loginState.password}
            onChange={(e) =>
              setLoginState({ ...loginState, password: e.target.value })
            }
          />
          <button onClick={handleLogin} className="btn-primary">
            Se connecter
          </button>
        </div>
      ) : (
        <div className="vpn-section">
          <h2>🔌 Connexion VPN</h2>
          <div className="status">
            <span className={`status-indicator ${isConnected ? "connected" : "disconnected"}`}>
              {isConnected ? "● CONNECTÉ" : "○ DÉCONNECTÉ"}
            </span>
          </div>
          {!isConnected ? (
            <button onClick={handleConnect} className="btn-connect">
              🚀 Connecter au VPN
            </button>
          ) : (
            <button onClick={handleDisconnect} className="btn-disconnect">
              🔴 Déconnecter
            </button>
          )}
          <button
            onClick={() => setToken(null)}
            className="btn-logout"
          >
            Se déconnecter
          </button>
        </div>
      )}

      {message && <div className="message">{message}</div>}

      <div className="features">
        <div className="feature-card">
          <h3>🛡️ Sécurité</h3>
          <p>Argon2 + JWT + TLS</p>
        </div>
        <div className="feature-card">
          <h3>⚡ Performance</h3>
          <p>WireGuard + PostgreSQL</p>
        </div>
        <div className="feature-card">
          <h3>🌐 P2P</h3>
          <p>Décentralisé & Libre</p>
        </div>
      </div>
    </div>
  );
}

export default App;
