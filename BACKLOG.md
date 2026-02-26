# Ghostline — Backlog post-MVP

> Ce fichier est la source de vérité pour les prochaines itérations.
> Les agents consultent ce fichier avant de commencer un run.
> Mettre à jour le statut au fil de l'implémentation.

---

## ✅ MVP — Complet (24 fév 2026)

| Phase | Contenu | Status |
|-------|---------|--------|
| P1 — Capture engine | Rust: ghostline-core, writer, frame, format .ghostline | ✅ DONE |
| P2 — Replay CLI + proxy | `ghostline replay <file>`, HTTP proxy, HIT/MISS | ✅ DONE |
| P3 — React viewer | Layout 3 panneaux, ReactFlow, drag & drop, dark theme | ✅ DONE |
| P4 — Python SDK | `ghostline.wrap(client)`, record/replay context managers | ✅ DONE |

---

## 🔴 Priorité 1 — Sécurité (GATE pour Show HN)

Ces items DOIVENT être faits AVANT toute publication publique (Show HN, awesome lists, etc.)

### [SEC-01] Fix replay proxy bind address
- **Qui:** DEV
- **Quoi:** `ghostline replay` bind sur `0.0.0.0` → changer en `127.0.0.1`
- **Fichier:** `crates/ghostline-cli/src/main.rs`
- **Effort:** 5 min
- **Status:** ⏳ TODO

### [SEC-02] Scrubbing layer — redact secrets dans les frames
- **Qui:** DEV + SECURITY (validation)
- **Quoi:** Avant d'écrire un frame dans le .ghostline, redacter automatiquement les patterns sensibles dans `request_bytes` et `response_bytes`
- **Patterns à redacter:** `sk-...`, `Bearer ...`, `api-key: ...`, `Authorization: ...`, clés AWS, etc.
- **Config:** opt-out via `ghostline.record(..., scrub=False)` pour usage local
- **Viewer:** toggle "Show raw" déjà prévu dans DetailPanel (off par défaut)
- **Status:** ⏳ TODO — bloqué sur threat model SECURITY (deadline 26 fév 18h)

### [SEC-03] Threat model SECURITY
- **Qui:** SECURITY
- **Quoi:** Document threat model complet pour Ghostline (attack surface, trust boundaries, mitigations)
- **Deadline:** 26 fév 18h UTC
- **Status:** 🔄 IN PROGRESS

---

## 🟡 Priorité 2 — Distribution

### [DIST-01] ~~Publish sur PyPI~~ ✅ DONE — pypi.org/project/ghostline/0.1.0
- **Qui:** DEV
- **Quoi:** `pip install ghostline` disponible publiquement
- **Comment:** `cd sdk && python -m build && twine upload dist/*`
- **Prérequis:** compte PyPI (Joseph doit créer + donner token), scrubbing P1 fait
- **Status:** ⏳ TODO

### [FEAT-00] Transparent proxy mode (PRIORITÉ HAUTE)
- **Qui:** DEV
- **Quoi:** `ghostline proxy --out ./runs/` — intercepte TOUS les appels LLM sans modifier le code
- **Pourquoi:** Fonctionne avec Claude Code, Cursor, LangChain, n'importe quel client. Zero code change.
- **Comment:** HTTP proxy local (port 9000) qui forward vers API réelle + enregistre dans .ghostline
- **Env vars:** `ANTHROPIC_BASE_URL=http://localhost:9000` ou `OPENAI_BASE_URL=http://localhost:9000`
- **Effort:** 1-2 jours (le replay proxy existe déjà — adapter en bidirectionnel)
- **Status:** ⏳ TODO — NEXT après threat model

### [DIST-02] Deploy viewer en ligne
- **Qui:** DEVOPS + DEV
- **Quoi:** `viewer.ghostline.dev` ou GitHub Pages — viewer accessible sans `npm run dev`
- **Comment:** `npm run build` → deploy sur Cloudflare Pages ou GitHub Pages
- **Status:** ⏳ TODO

### [DIST-03] Show HN
- **Qui:** GROWTH
- **Quoi:** Post "Show HN: Ghostline — deterministic replay for AI agents"
- **Gate:** SEC-01 + SEC-02 + SEC-03 validés + PyPI live + README finalisé
- **Hook:** "Record once, replay without tokens, debug by time-traveling through any state"
- **Status:** 🔒 BLOQUÉ sur sécurité

### [DIST-04] Awesome lists PRs
- **Qui:** GROWTH
- **Repos cibles:** `awesome-llm-apps`, `awesome-ai-agents`, `awesome-rust`
- **Status:** ⏳ TODO (après Show HN)

---

## 🟢 Priorité 3 — Features post-MVP

### [FEAT-01] Branching — fork à step N
- **Qui:** DEV
- **Quoi:** `ghostline fork <file> --at <step>` → nouveau `.ghostline` avec `parent_run_id` + `fork_at_step` dans le header
- **Viewer:** clic droit sur nœud → "Fork from here ⑂", shortcut `B`
- **Status:** ⏳ TODO

### [FEAT-02] Multi-provider
- **Qui:** DEV
- **Quoi:** Wrapper Python pour OpenAI + LiteLLM (en plus d'Anthropic)
- **Status:** ⏳ TODO

### [FEAT-03] Zoom sémantique viewer
- **Qui:** DESIGN + DEV
- **Quoi:** Dézoomé → phases groupées. Zoomé → step-by-step individuel
- **Status:** ⏳ TODO

### [FEAT-04] Export partageable
- **Qui:** DEV
- **Quoi:** `ghostline export --format html` → fichier HTML standalone avec viewer embarqué
- **Viralité:** chaque replay partagé = démo live du produit
- **Status:** ⏳ TODO

---

## 🎬 Priorité 4 — Marketing

### [MKT-01] Demo video
- **Qui:** REMOTION
- **Quoi:** Vidéo courte (60s) : enregistre un run réel → rejoue → montre "0 tokens spent"
- **Stack:** Playwright screencast + FFmpeg ou Remotion
- **Status:** ⏳ TODO (après viewer déployé)

### [MKT-02] Twitter/X thread technique
- **Qui:** GROWTH
- **Quoi:** Thread walkthrough — comment ça marche sous le capot (format binaire, zstd, O(1) index)
- **Timing:** même jour que Show HN
- **Status:** ⏳ TODO

---

## 📐 Règles pour les agents

1. **GATE Show HN**: SEC-01 + SEC-02 + SEC-03 validés AVANT publication
2. **Commits**: `feat:`, `fix:`, `docs:`, `refactor:` — pas de mention Claude/AI/LLM
3. **Pas de SaaS** sans approval Joseph écrit
4. **Branching MVP scope**: inclus si simple, sinon post-MVP
5. **PyPI token**: Joseph doit le fournir — les agents ne créent pas de comptes

---

*Dernière mise à jour: 2026-02-24 par CEO*

---

## 🔵 Priorité 5 — Intelligence (post-v1)

### [INTEL-01] Zvec — Recherche sémantique dans les replays
- **Qui:** DEV
- **Quoi:** Indexation vectorielle des frames .ghostline via [alibaba/zvec](https://github.com/alibaba/zvec)
- **Pourquoi:** Chercher "trouve le step où l'agent a halluciné" au lieu de scroller frame par frame
- **Comment:**
  - Embedder chaque frame (request + response) via un modèle léger (e5-small ou nomic-embed)
  - Indexer avec Zvec (in-process, Proxima backend)
  - API: `ghostline search run.ghostline "prompt injection attempt"`
  - Viewer: barre de recherche sémantique dans la sidebar
- **Effort:** 1 semaine
- **Status:** ⏳ TODO — post proxy transparent
