# 🛠 WorldVPN Operations & Scripts

<div align="center">

<br/>

```
███████╗ ██████╗██████╗ ██╗██████╗ ████████╗███████╗
██╔════╝██╔════╝██╔══██╗██║██╔══██╗╚══██╔══╝██╔════╝
███████╗██║     ██████╔╝██║██████╔╝   ██║   ███████╗
╚════██║██║     ██╔══██╗██║██╔═══╝    ██║   ╚════██║
███████║╚██████╗██║  ██║██║██║        ██║   ███████║
╚══════╝ ╚═════╝╚═╝  ╚═╝╚═╝╚═╝        ╚═╝   ╚══════╝
```

**Automation, Deployment & Maintenance**

<br/>

![Shell](https://img.shields.io/badge/Language-Bash-4EAA25?style=flat-square&logo=gnu-bash)
![Python](https://img.shields.io/badge/Language-Python-3776AB?style=flat-square&logo=python)
![Docker](https://img.shields.io/badge/Tool-Docker-2496ED?style=flat-square&logo=docker)

</div>

## 📖 Overview
The `scripts` directory contains essential tools for managing the lifecycle of WorldVPN. From automated builds to database management and deployment orchestration on Render, these scripts ensure that the system stays operational with minimal manual intervention.

## 🛠 Key Scripts

### 📦 `deploy-render.sh`
Automates the push and deployment sequence to Render, ensuring that migrations are synced and the environment is healthy.

### 🧪 `test-p2p-network.sh`
Orchestrates a local multi-node P2P cluster for rigorous performance and discovery testing.

### 🧹 `db-maintenance.sh`
Handles database backups, pruning of expired sessions, and schema health checks.

## 🚀 Usage
Most scripts are designed to be run from the project root. Ensure you have the necessary permissions:
```bash
chmod +x scripts/*.sh
./scripts/test-p2p-network.sh
```

---
**WorldVPN** · Private. Decentralized. Transparent.
