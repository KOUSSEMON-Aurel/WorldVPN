# ADR-002 — Design Zero-Log et anonymat par défaut

**Date**: 2025-01-01  
**Statut**: Accepté  
**Auteur**: WorldVPN Team

---

## Contexte

WorldVPN est un VPN à anonymat radical. Contrairement aux VPNs traditionnels, il ne peut pas se permettre de stocker des logs qui pourraient être compromis, saisis par des autorités, ou fuiter. Le design doit être tel que même en ayant un accès complet à la base de données de production, il est impossible d'établir un lien entre une session et une identité dans le monde réel.

## Principes directeurs

### 1. Pas de PII (Personally Identifiable Information)
- Aucun email, numéro de téléphone, ou adresse IP n'est jamais enregistré.
- L'identité est uniquement une clé publique Ed25519 (`ed25519:<hex>`).

### 2. Authentification sans connaissance d'identité
- Flux: Le client signe un timestamp avec sa clé privée → Le serveur vérifie la signature → JWT éphémère émis.
- Le serveur ne voit que la clé publique et la signature. Il n'apprend rien sur qui est l'utilisateur.

### 3. Sessions éphémères (TTL 24h)
- Toutes les sessions (`vpn_sessions`) ont un TTL de 24 heures.
- Un service de `pruning` (`services/pruning.rs`) tourne en continu et supprime les sessions expirées.
- **Garantie**: Aucune session identifiable ne subsiste en base après 24h d'inactivité.

### 4. Métriques anonymisées
- Seuls des compteurs agrégés sont conservés (nombre de sessions, bytes relayés).
- Aucune valeur de métrique n'est associable à une identité ou IP.
- Les métriques sont exposées uniquement en interne (Prometheus port 9090).

## Schéma de la base de données

Les tables suivantes existent:
- `users`: `id`, `public_key_hex`, `username` (généré), `created_at`. Pas d'email.
- `vpn_sessions`: `id`, `user_id`, `connected_at`, `disconnected_at`, `expires_at`. Pas de destination, pas d'IP source.
- `credit_transactions`: `id`, `user_id`, `amount`, `tx_type`. Pas de contenu de trafic.

## Vérification et audit

- Le service de pruning doit faire l'objet d'un test d'intégration qui vérifie qu'après expiration, les données sont bien supprimées.
- Un audit externe du schéma PostgreSQL et des migrations est recommandé avant le lancement en production.
- `cargo audit` doit être exécuté à chaque PR pour détecter les CVEs dans les dépendances.

## Limites reconnues

- Les nœuds communautaires qui relaient du trafic voient les paquets chiffrés, mais pas leur contenu.
- Le backend voit quels utilisateurs se connectent (et à quelle fréquence), mais pas vers quelles destinations.
- La recommandation est d'utiliser un relais Tor ou VLESS/Hysteria2 pour les cas d'usage nécessitant une anonymisation complète vis-à-vis du backend.
