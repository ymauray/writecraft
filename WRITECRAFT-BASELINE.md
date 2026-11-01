# WriteCraft — base de référence des tests

> Relevé le 2026-10-31 sur le Mac (macOS 27.0.1, build 26A434), cargo-nextest 0.9.150, avec
> `CRAFT_I18N_DIR` défini. Mis à jour le 2026-11-01 après la fusion de WordCraft `a88e0052`
> (commit `65abb041`) ; l'état de départ (commit `69e5ec0b`) est rappelé en dessous.
> Règle : une modification ne doit ajouter **aucun** échec ni test ignoré à cette liste.

## Commande

`cargo xtask test` (et donc `cargo xtask ci`) s'arrête au premier échec : son total varie d'une
exécution à l'autre (2082 ou 2085 réussis selon le moment de la coupure) et ne sert pas de
référence. La référence vient d'une exécution complète, avec la même sélection que xtask :

```sh
cargo nextest run --workspace --lib --bins --tests \
  -E 'not (package(wordcraft-ui-egui) & test(=i18n::audit::untranslated_interface_text))' \
  --no-fail-fast
cargo xtask i18n-report
cargo test --workspace --doc
```

## Résultat

| Étape | Actuel (`65abb041`) | Départ (`69e5ec0b`) |
|---|---|---|
| Tests (nextest, `--no-fail-fast`) | 4726 lancés : **4724 réussis, 2 échoués, 5 ignorés** (environ 1 min 40, plus 1 min 35 pour l'audit) | 4590 lancés : 4588 réussis, 2 échoués, 5 ignorés |
| Audit i18n (`i18n-report`) | ✅ réussi ; voir « Traduction française » | ✅ réussi |
| Doctests | ✅ aucun échec | ✅ aucun échec |
| fmt, clippy (`--workspace --all-targets -D warnings`) | ✅ | pas relancés |
| assets, layers | ✅ (57 fichiers attribués ; 20 crates, aucune violation) | pas relancés |
| wasm | non suivi : pas de version web pour WriteCraft | pas relancé |

Avant la fusion de WordCraft `a88e0052` (commit `5e0c4fe6`) : 4612 lancés, 4610 réussis, les
mêmes 2 échecs et 5 ignorés. Les 114 tests en plus viennent de WordCraft.

Les 22 tests en plus du départ sont les tests WriteCraft (`crates/engine/src/writecraft.rs`,
`crates/ui-egui/src/writecraft.rs`, `crates/ui-egui/src/pomodoro.rs`, `pomodoro/sound.rs`,
`word_tally.rs`). L'audit des traductions passe aussi. Trois tests de WordCraft ont été adaptés aux boutons
supprimés pour tout le monde (Partager, Discord) : `ribbon::tests::tab_strip_never_overlaps_the_right_buttons`,
`ribbon::tests::the_tab_strip_buttons_have_names`, `tests::the_default_app_button_shows_only_with_the_hosts_hook`.

## Échecs connus (2)

| Test | Message | Cause probable |
|---|---|---|
| `wordcraft-fonts fontdb::tests::a_thai_run_with_latin_falls_back_to_one_face` | `Thonburi shapes Thai` (`crates/fonts/src/fontdb/tests.rs:115`) | la police système Thonburi de macOS 27 n'a pas de table `GSUB` (`has_layout_tables`) ; dépend des polices installées |
| `wordcraft-ui-egui canvas::tests::a_right_to_left_balloon_reads_from_the_right` | `wrapped glyph 'ش': origin 358, max 44.341774, row width 41.9375, limit 400` (`crates/ui-egui/src/canvas/tests.rs:1157`) | rendu du texte arabe avec les polices du Mac ; non analysé. Échoue aussi lancé seul : échec stable, pas un test instable |

Ces deux tests dépendent des polices système : ils peuvent passer sur une autre machine.

## Tests ignorés (5)

| Test | Raison |
|---|---|
| `wordcraft-engine::it recovery_format::native_json_size_with_a_large_picture` | `#[ignore = "measurement"]` |
| `wordcraft-fonts fontdb::tests::kai_contours_include_complete_character_body` | nécessite `kaiu.ttf` sous licence (`WORDCRAFT_TEST_KAI_FONT`) |
| `wordcraft-ui-egui mini_toolbar::tests::the_bar_scrolls_under_the_ribbon_not_over_it` | en conflit avec l'ancre de sélection de #1501 |
| `wordcraft-ui-egui tests::perf::big_table_scroll_bench` | banc de performance (`#[ignore]`) |
| `wordcraft-ui-egui i18n::audit::untranslated_interface_text` | exclu de la passe principale, lancé à part par `cargo xtask i18n-report` (réussi) |

## Traduction française

Les traductions viennent de [storytold/craft-i18n](https://github.com/storytold/craft-i18n), au
commit de `craft-i18n.rev` (`9205108c`), cloné dans `../craft-i18n`. Sans `CRAFT_I18N_DIR`,
l'interface est en anglais seulement.

```sh
CRAFT_I18N_DIR=/Volumes/EVO_PRO_1T/Development/craft-i18n cargo xtask i18n-report
```

Relevé du 2026-11-01 pour `fr` : **36 textes sans traduction** (23 avant la fusion de WordCraft
`a88e0052`, 25 au départ) et 74 textes volontairement identiques à l'anglais (73 avant la fusion,
74 au départ ; « Normal », « Zoom », « Options », « OK »,
noms propres…, ce qui est normal). Les deux textes de « Check for Updates » ont disparu avec le
bouton.

Les 13 textes arrivés avec la fusion viennent de WordCraft, et craft-i18n ne les traduit pas
encore en français (vérifié sur sa branche principale le 2026-11-01) :

- couleurs de surlignage (#5008) : Bright Green, Dark Blue, Dark Red, Dark Yellow, Gray-25%,
  Gray-50%, Teal, Turquoise ;
- fenêtre Macros (Affichage › Macros, #4327), **visible dans WriteCraft** : Macro name:, Record…,
  Run, There are no macros yet. Record one to add it here. ;
- bibliographie (#4848) : Corporate Author.

Textes sans traduction française déjà présents avant la fusion (23) :

- Accept All Changes and Stop Tracking
- Accept and Move to Next
- Advanced Properties…
- Close Document
- Dock Left
- Dock Right
- Float
- Move Pane: {title}
- Page color
- Pane Options
- Reading Page Color
- Reject All Changes and Stop Tracking
- Reject and Move to Next
- Resize Pane
- Scale Font Sizes
- Separate text with
- Spanish (Latin America)
- Spanish (Uruguay)
- Styles Pane Position
- Switch\nWindows
- Switch Windows
- View and edit the document's custom properties.
- {name} options

La fenêtre de mise à jour au démarrage, qui restait en anglais, n'apparaît plus (commit
`1fdc4cdd`).

Règle : une modification ne doit pas allonger cette liste ; un nouveau texte d'interface
reçoit sa traduction française (dans craft-i18n, ou dans un catalogue propre à WriteCraft).
