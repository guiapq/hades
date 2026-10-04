# Project Hades: Agent Guidelines & Policy

## 1. Discretion & Trademark Policy
- **No Proprietary Trademarks or Brand Names:** Never cite or mention commercial game titles, trademarks, company names, publishers, or official server brands anywhere in code, commits, documentation, or discussions.
- **No Offensive or Adversarial Language:** Never use hostile, disparaging, or provocative language towards any publisher, company, copyright holder, or studio. Project Hades is a clean-room, educational, high-performance 2D/2.5D MMORPG engine built from scratch.
- **Discreet Technical Vocabulary:**
  - Use neutral, generic phrasing: "Classic 2.5D MMORPG", "Heritage Pre-Renewal mechanics", "Classic isometric RPG formulas", "Legacy reference emulators", "Legacy .gat cell grids".
  - Attribute mechanics purely to mathematical models and open specifications.

## 2. Engineering Principles
- **Potato Budget:** The engine must run 500 CCU on a $4/month VPS (1 vCPU, 512MB RAM).
- **Zero Allocations on Gameplay Ticks:** No dynamic heap allocation (`Box`, `Vec`, `String`, etc.) inside tick loops, collision checks, combat calculations, or AoI multicast.
- **Modern Transport:** Exclusively QUIC / WebTransport over TLS 1.3. No legacy synchronous TCP.
- **Spec-Driven Development (SDD/TDD):** Every feature or protocol must have a written RFC in `specs/` before implementation, backed by comprehensive unit tests.
