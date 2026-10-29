# WriteCraft — vue d'ensemble

> Rédigé le 2026-10-29. WriteCraft est un fork de [WordCraft](https://github.com/storytold/wordcraft)
> (`upstream`) hébergé sur [ymauray/writecraft](https://github.com/ymauray/writecraft) (`origin`).
> Objectif : un traitement de texte épuré, orienté écriture, qui reste synchronisé avec WordCraft.

## Vue d'ensemble

- **Le projet** : un traitement de texte libre en Rust (licence MIT/Apache), qui vise la parité avec
  Word. Il tourne sur macOS, Windows, Linux, BSD et dans le navigateur (WASM). Le code fait environ
  85 000 lignes réparties en 15 crates (`docs/architecture.md`).
- **Son principe** : chaque action est une « commande » identifiée par un nom (422 au total, dans
  `crates/engine/src/cmd/*.rs`). Le ruban, les raccourcis, la ligne de commande et le serveur MCP
  appellent tous ces mêmes commandes.
- **L'interface** est séparée du reste, dans `crates/ui-egui`. Le ruban se trouve dans
  `crates/ui-egui/src/ribbon/`, avec un fichier par onglet : `home.rs`, `insert.rs`, `mailings.rs`,
  `references.rs`, `review.rs`, etc.
- **Le travail récent côté WordCraft** (au moment du fork, commit `aadb077e`) a surtout porté sur la
  mise en conformité avec Clippy (`c4fafc21`, `ac2cc0c4`), puis sur la sortie de la version 0.12.0
  (`3de6ff95`) et sur le packaging (`aadb077e`, `9724f15c`).

## Idée 1 : épurer le ruban

C'est faisable, et plus simple que prévu :

- La liste des onglets est définie à un seul endroit, `crates/engine/src/ribbon.rs` (`TABS`). Il y
  en a 13 : File, Home, Insert, Draw, Design, Layout, References, Mailings, Review, View, Developer,
  Zotero, Help.
- **La personnalisation du ruban existe déjà** : chaque onglet (`TabEntry`) a un champ `hidden`. On
  peut donc masquer des onglets par défaut sans supprimer de code.
- **Recommandation** : masquer les onglets et groupes inutiles dans l'interface, mais garder les
  commandes du moteur. Supprimer des commandes ferait échouer plusieurs garde-fous, en particulier
  le test qui vérifie le taux de parité avec Word (il ne doit jamais baisser) et les tests
  d'interface (`crates/ui-egui/src/tests.rs`, environ 7 000 lignes). Garder les commandes évite de
  casser ces tests, limite les conflits lors des synchronisations avec WordCraft et laisse ouverte
  la possibilité de réactiver un onglet plus tard.

## Idée 2 : ajouts (pomodoro, compteur de mots et signes par session)

Les bases sont déjà là :

- **Barre d'état** : `crates/ui-egui/src/chrome.rs` (`status_bar`) affiche déjà le nombre de mots,
  avec un cache (`cached_word_count`) et une boîte de dialogue « wordCount ».
- **Horloge** : `crates/ui-egui/src/clock.rs` fournit une horloge monotone (`now_ms`) qui fonctionne
  aussi en WASM. C'est ce qu'il faut pour un minuteur pomodoro.
- **Comptage de mots** : `wordcraft_doc::count_words` existe déjà. Pour un compteur par session, il
  suffit de mémoriser les compteurs de mots et de signes à l'ouverture, puis d'afficher la
  différence.
- **Règles à respecter** (`CLAUDE.md`) : chaque nouvelle fonction est une commande (par exemple
  `pomodoro.start`) accompagnée de tests ; aucun code ne doit pouvoir faire planter l'application
  (pas de `unwrap`, `expect`, `panic!`…).

## Synchronisation avec WordCraft

On reste synchronisé avec WordCraft : on préfère donc masquer plutôt que supprimer, et isoler les
ajouts dans des fichiers propres à WriteCraft quand c'est possible.

```sh
git fetch upstream
git merge upstream/main     # ou: git rebase upstream/main
git push origin main
```

## Où développer et tester

Le développement et les tests se font sur le Mac (avec interface graphique). Le serveur Ubuntu,
sans écran, sert seulement à explorer le code.

- `cargo xtask ci` (fmt, clippy, tests, i18n, doctests, assets, couches, wasm) avant chaque commit.
- Prérequis : la cible `wasm32-unknown-unknown` (`rustup target add wasm32-unknown-unknown`) ;
  `cargo-nextest` est recommandé (`cargo install cargo-nextest --locked`), sinon `xtask` se rabat
  sur `cargo test`.
- Sans écran, il reste les tests d'interface (`crates/ui-egui/src/tests.rs`, egui_kittest) et
  `cargo run --release -p wordcraft-ui-egui --example ui_shot -- script.jsonl`, qui fait des
  captures d'écran hors écran.

## État de départ (CI sur le serveur Ubuntu, 2026-10-29, commit `11109377`)

| Étape | Résultat |
|---|---|
| fmt | ✅ |
| clippy | ✅ (environ 1 min 15 s) |
| tests (nextest) | 4 558 réussis, 1 échoué, 5 ignorés sur 4 559 lancés (le premier échec a arrêté les 44 restants) ; une exécution complète prend 1 min 30 environ |
| doctests, assets, layers | ✅ lors d'une exécution étape par étape |
| i18n-report, wasm | pas lancés jusqu'au bout |

Les échecs observés viennent du serveur, pas du code :

- `backstage::tests::recovery_page_keeps_dismissed_copies_and_confirms_discard` : délai de 5 s
  dépassé quand la machine est chargée ; passe 3 fois sur 3 lancé seul.
- `wordcraft-zotero tests::cancelled_session_and_no_zotero` : le port du faux serveur Zotero était
  aussi utilisé par un processus root de la machine.
- Une mise à jour système qui remplace `/dev/null` par un fichier ordinaire casse la compilation
  (`failed to run rustc to learn about target-specific information`) ; après correction, supprimer
  `target/.rustc_info.json`.

Il reste à confirmer une CI entièrement verte sur le Mac.

## Ce qui manque

- Le dossier `plan/` (statut et plan d'exécution) est absent : exclu de git, il n'existe que sur la
  machine de l'auteur d'origine. `ROADMAP.md` et `docs/` le remplacent.
- Pas encore lus : `docs/roadmap.md`, `docs/gaps.md`, `AGENTS.md`, le dépôt `craftrules`.
