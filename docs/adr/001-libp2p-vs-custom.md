# ADR-001 — Utilisation de libp2p pour le réseau P2P

**Date**: 2025-01-01  
**Statut**: Accepté  
**Auteur**: WorldVPN Team

---

## Contexte

WorldVPN nécessite un mécanisme de découverte de nœuds décentralisé, sans tracker central, pour respecter son engagement de confidentialité et de résilience. Deux approches principales ont été étudiées.

## Options envisagées

### Option A — Implémentation custom (UDP + DHT maison)
- **Avantages**: Contrôle total, binaire minimal.
- **Inconvénients**: Temps de développement élevé (6-12 mois), vecteurs de sécurité non couverts, pas de NAT traversal natif.

### Option B — `libp2p` (Rust)
- **Avantages**:
  - Protocoles éprouvés en production (IPFS, Ethereum, Filecoin).
  - Kademlia DHT pour la découverte distribuée, sans point central.
  - Gossipsub pour la propagation des reçus de crédit et annonces de nœuds.
  - mDNS pour la découverte locale (LAN, tests).
  - Authentification des messages (Signed mode).
  - NAT traversal (STUN/TURN via composants WebRTC séparés).
- **Inconvénients**: Binaire plus lourd (~4 MB compilé), courbe d'apprentissage.

## Décision

**Option B — libp2p** a été retenu.

## Conséquences

- Le nœud P2P est initialisé dans `crates/vpn-core/src/p2p.rs`.
- Les topics Gossipsub utilisés sont:
  - `worldvpn/nodes/v1` — annonces de nœuds.
  - `worldvpn/credits/v1` — reçus de crédit signés.
- Le protocole Kademlia utilise le namespace `/worldvpn/kad/1.0.0`.
- Tous les messages Gossipsub doivent être signés (`MessageAuthenticity::Signed`).
- Les `CreditReceipt` doivent être vérifiés par signature Ed25519 avant tout traitement.
