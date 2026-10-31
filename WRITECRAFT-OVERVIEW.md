# WriteCraft — vue d'ensemble

> Rédigé le 2026-10-29, mis à jour le 2026-10-31 (section « Ce qui est fait »). WriteCraft est un fork de [WordCraft](https://github.com/storytold/wordcraft)
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

## Ce qui est fait (2026-10-31)

Toutes les modifications sont commitées sur `main` (commits `1fdc4cdd` à `5808ce8a`).

### Comment c'est construit

Pour rester synchronisé avec WordCraft, les modifications **masquent** sans supprimer de
commandes, et sont regroupées dans deux fichiers propres à WriteCraft :

- **`crates/ui-egui/src/writecraft.rs`** : les simplifications de l'interface, derrière un
  interrupteur (`writecraft::set_trimmed(true)`) que l'application de bureau
  (`apps/wordcraft/src/main.rs`) et l'application web (`apps/wordcraft-web/src/web.rs`) activent au
  démarrage. Il contient les listes d'onglets, de blocs et de commandes masqués, et
  `apply_app_defaults` (correcteur désactivé).
- **`crates/engine/src/writecraft.rs`** : les valeurs par défaut des nouveaux documents
  (`apply_defaults`), activées par `Session::writecraft_defaults`, que les deux applications activent
  aussi.
- Sans ces interrupteurs (tests hérités de WordCraft, exemples, ligne de commande, MCP), on garde le
  comportement de WordCraft. Les tests de WordCraft restent donc tels quels, et les tests WriteCraft
  (dans les deux fichiers) vérifient les deux modes.
- Les fichiers de WordCraft ne changent qu'à quelques endroits, par de petites fonctions
  d'enrobage (`group`, `big`, `small` dans `ribbon/home.rs` et `ribbon/review.rs`) ou une ligne
  d'appel.

### Modifications

| Domaine | Changement |
|---|---|
| Mise à jour | Plus de vérification au démarrage ni de bouton « Check for Updates » : le module ne connaît que les versions de WordCraft. |
| Barre de titre et onglets | Plus de bouton Discord (ni sa case dans Options), plus de bouton Partager. |
| Aide et Fichier | Plus d'entrée Discord (onglet Aide, KeyTip, lien dans Fichier). |
| Onglets | Seuls restent Fichier, Accueil, Révision, Affichage, Aide. Masqués : Insertion, Dessin, Création, Mise en page, Références, Publipostage, Développeur, Zotero. |
| Accueil | Presse-papiers ; Police réduite à Gras, Italique, Souligné ; Styles sans bouton de volet ; Modification. Masqués : Paragraphe, Voix, Éditeur. |
| Révision | Plus de Grammaire et orthographe, de Thésaurus ni de bloc Langue (Traduire, Langue). |
| Correcteur | Désactivé au démarrage (les outils de correction n'ont pas le français), même si une préférence enregistrée l'activait. |
| Nouveaux documents | Texte en français (`fr-FR`). Galerie de styles : Normal, Titre 1, Titre, Ellipse. |

Styles des nouveaux documents (les documents existants gardent les leurs) :

| Style | Police | Paragraphe |
|---|---|---|
| Normal | Georgia 14 pt, couleur auto | justifié, retrait 1re ligne 0,5 cm, 0 pt avant, 6 pt après, interligne 1,5 |
| Titre 1 | Georgia gras 20 pt, couleur auto | à gauche, retrait 1re ligne 0,5 cm, 24 pt avant, 12 pt après, interligne 1,5 |
| Titre | Georgia 22 pt, couleur auto | centré, sans retrait, 26 pt avant et après, interligne simple |
| Ellipse (nouveau) | comme Normal | centré, sans retrait, 12 pt avant et après, interligne simple |

Georgia est une police système de macOS et Windows ; sous Linux, une police serif libre la
remplace. Elle n'est pas intégrée au dépôt.

### Façon de travailler

- On commite directement sur `main`.
- Lancer l'application en profil `dev`, avec les traductions :
  `CRAFT_I18N_DIR=/Volumes/EVO_PRO_1T/Development/craft-i18n cargo run -p wordcraft -- --control 7981`
  (environ 10 à 30 s après une modification ; `--release` prend 1 min 30 ou plus).
- `craft-i18n` est cloné dans `../craft-i18n`, au commit de `craft-i18n.rev`. Sans
  `CRAFT_I18N_DIR`, l'interface est en anglais, et alterner avec et sans la variable recompile
  l'interface à chaque fois.
- Par modification : compiler, relancer, regarder. Avant un commit : clippy et les tests de
  l'interface sans l'audit des traductions
  (`cargo nextest run -p wordcraft-ui-egui -p wordcraft -E 'not test(=i18n::audit::untranslated_interface_text)'`,
  environ 30 s). La suite complète et la référence : `WRITECRAFT-BASELINE.md`.

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

Sur le Mac (2026-10-31), les 2 seuls échecs viennent des polices système (Thai et arabe), et la
vérification wasm ne peut pas tourner sans `rustup` : voir `WRITECRAFT-BASELINE.md`.

## Ce qui manque

- La vérification wasm (`cargo xtask wasm`) : la cible `wasm32-unknown-unknown` n'est pas installée
  (Rust de Homebrew, sans `rustup`).
- Les documents de progression de WordCraft (`ROADMAP.md`, `docs/`) ne décrivent pas les
  modifications WriteCraft ; elles sont suivies ici.
- Le dossier `plan/` (statut et plan d'exécution) est absent : exclu de git, il n'existe que sur la
  machine de l'auteur d'origine. `ROADMAP.md` et `docs/` le remplacent.
- Pas encore lus : `docs/roadmap.md`, `docs/gaps.md`, `AGENTS.md`, le dépôt `craftrules`.
