# WriteCraft — base de référence des tests

> Relevé le 2026-10-31 sur le Mac (macOS 27.0.1, build 26A434), cargo-nextest 0.9.150, avec
> `CRAFT_I18N_DIR` défini. Mis à jour le 2026-11-20 après la fusion de WordCraft `6ef13b2c`
> (0.18.1) ; l'état de départ (commit `69e5ec0b`) est rappelé en dessous.
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

| Étape | Actuel (fusion `6ef13b2c`) | Départ (`69e5ec0b`) |
|---|---|---|
| Tests (nextest, `--no-fail-fast`) | 5316 lancés : **5300 réussis, 16 échoués, 10 ignorés** (environ 2 min 40, plus 5 min pour l'audit) | 4590 lancés : 4588 réussis, 2 échoués, 5 ignorés |
| Audit i18n (`i18n-report`) | ✅ réussi ; voir « Traduction française » | ✅ réussi |
| Doctests | ✅ aucun échec | ✅ aucun échec |
| fmt, clippy (`--workspace --all-targets -D warnings`) | ✅ | pas relancés |
| assets, layers | ✅ (55 fichiers attribués ; 18 crates, aucune violation) | pas relancés |
| wasm | non suivi : pas de version web pour WriteCraft | pas relancé |

Après la fusion de WordCraft `a88e0052` (commit `65abb041`) : 4726 lancés, 4724 réussis, 2 échoués,
5 ignorés. Les 14 nouveaux échecs et les 5 nouveaux tests ignorés de la fusion `6ef13b2c` viennent
tous de WordCraft : les mêmes échouent sur `upstream/main` seul, sur ce Mac (vérifié le 2026-11-20
dans un worktree).

Avant la fusion de WordCraft `a88e0052` (commit `5e0c4fe6`) : 4612 lancés, 4610 réussis, les
mêmes 2 échecs et 5 ignorés. Les 114 tests en plus viennent de WordCraft.

Les 22 tests en plus du départ sont les tests WriteCraft (`crates/engine/src/writecraft.rs`,
`crates/ui-egui/src/writecraft.rs`, `crates/ui-egui/src/pomodoro.rs`, `pomodoro/sound.rs`,
`word_tally.rs`). L'audit des traductions passe aussi. Trois tests de WordCraft ont été adaptés aux boutons
supprimés pour tout le monde (Partager, Discord) : `ribbon::tests::tab_strip_never_overlaps_the_right_buttons`,
`ribbon::tests::the_tab_strip_buttons_have_names`, `tests::the_default_app_button_shows_only_with_the_hosts_hook`.
Depuis la fusion `6ef13b2c`, `i18n::audit::the_arabic_interface_is_mirrored_and_never_panics` vise
le bouton Modification au lieu de Partager.

## Échecs connus (16)

| Test | Message | Cause probable |
|---|---|---|
| `wordcraft-engine::strict_export` (12 tests) et `wordcraft-cli::it strict::` (2 tests) : `strict_refuses_an_uncovered_character_and_writes_nothing`, `an_uncovered_code_point_fails_strict_with_a_report_and_no_file`… | `the host has faces covering all of plane 4` | les polices du Mac couvrent tout le plan 4 d'Unicode : le test ne trouve pas de caractère sans police. Échoue aussi sur WordCraft seul |
| `wordcraft-ui-egui theme::tests::interface_symbols_have_glyphs` | `symbols without a glyph: ["。 U+3002 in tests.rs"]` | police CJK absente des polices de l'interface ; échoue aussi sur WordCraft seul |
| `wordcraft-ui-egui panes::tests::navigation_tabs_fit_the_narrowest_pane_in_every_language` | `No glyph for "ا" (U+0627) in Proportional` | police arabe absente des polices de l'interface ; échoue aussi sur WordCraft seul |
| `wordcraft-fonts fontdb::tests::a_thai_run_with_latin_falls_back_to_one_face` | `Thonburi shapes Thai` (`crates/fonts/src/fontdb/tests.rs:115`) | la police système Thonburi de macOS 27 n'a pas de table `GSUB` (`has_layout_tables`) ; dépend des polices installées |
| `wordcraft-ui-egui canvas::tests::a_right_to_left_balloon_reads_from_the_right` | `wrapped glyph 'ش': origin 358, max 44.341774, row width 41.9375, limit 400` (`crates/ui-egui/src/canvas/tests.rs:1157`) | rendu du texte arabe avec les polices du Mac ; non analysé. Échoue aussi lancé seul : échec stable, pas un test instable |

Ces deux tests dépendent des polices système : ils peuvent passer sur une autre machine.

## Tests ignorés (10)

| Test | Raison |
|---|---|
| `wordcraft-engine::it recovery_format::native_json_size_with_a_large_picture` | `#[ignore = "measurement"]` |
| `wordcraft-fonts fontdb::tests::kai_contours_include_complete_character_body` | nécessite `kaiu.ttf` sous licence (`WORDCRAFT_TEST_KAI_FONT`) |
| `wordcraft-ui-egui mini_toolbar::tests::the_bar_scrolls_under_the_ribbon_not_over_it` | en conflit avec l'ancre de sélection de #1501 |
| `wordcraft-ui-egui tests::perf::big_table_scroll_bench` | banc de performance (`#[ignore]`) |
| `wordcraft::bin/wordcraft print::isolated::tests::helper_process` | lancé par les tests du processus d'impression |
| `wordcraft-layout display::page::tests_warp_lookups::text_box_warp_lookups_do_not_rescan_the_paragraph_prefix` | banc de performance |
| `wordcraft-layout display::page::tests_warp_lookups::text_box_warp_lookups_stop_at_the_last_valid_requested_offset` | banc de performance |
| `wordcraft-render tests::huge_metafile_repaint_time` | mesure de temps |
| `wordcraft-ui-egui proofing_pane::tests::spelling_review_dialog_preview` | rendu hors écran, demande un adaptateur graphique |
| `wordcraft-ui-egui i18n::audit::untranslated_interface_text` | exclu de la passe principale, lancé à part par `cargo xtask i18n-report` (réussi) |

## Traduction française

Les traductions viennent de [storytold/craft-i18n](https://github.com/storytold/craft-i18n), au
commit de `craft-i18n.rev` (`9205108c`), cloné dans `../craft-i18n`. Sans `CRAFT_I18N_DIR`,
l'interface est en anglais seulement.

```sh
CRAFT_I18N_DIR=/Volumes/EVO_PRO_1T/Development/craft-i18n cargo xtask i18n-report
```

Relevé du 2026-11-20 pour `fr`, après la fusion de WordCraft `6ef13b2c` : **103 textes sans
traduction** (36 avant cette fusion, 25 au départ) et 74 textes volontairement identiques à
l'anglais (« Normal », « Zoom », « Options », « OK », noms propres…, ce qui est normal).

Les 67 textes en plus viennent tous de WordCraft (paquets de langue, polices manquantes, outils de
paragraphe, Markdown, défilement, fenêtres côte à côte…) : craft-i18n ne les traduit pas encore en
français. On ne les traduit pas dans WriteCraft ; ils arriveront par WordCraft. Certains ne se
voient pas dans WriteCraft (« Paragraph Tools » est masqué), d'autres si : la fenêtre des polices
manquantes, qui parle aussi de « WordCraft » (« restart WordCraft », « Then add it to WordCraft »).

Liste complète (sortie de `cargo xtask i18n-report`) :

- Accept All Changes and Stop Tracking
- Accept and Move to Next
- Add Blank Paragraphs Between
- Arrange All
- Bright Green
- By:
- Close Document
- Convert Indents to Spaces
- Convert Line Breaks to Paragraphs
- Convert Spaces to Indentation
- Dark Blue
- Dark Red
- Dark Yellow
- Define New Bullet…
- Delete Blank Paragraphs
- Delete Blank Paragraphs in the Whole Document
- Delete Previous Sentence
- Document background
- Document fitted to 1 page (font: -2.0 pt, spacing: 88%).
- Don't show this again
- Download from {publisher}
- Download the font. If it comes as a .zip file, unpack it.
- Emphasis Mark
- Error! Bookmark not defined.
- Error! Reference source not found.
- Fit\nOne Page
- Fit One Page
- Flat OpenDocument Text (*.fodt)
- Fonts come with licence terms. Read the publisher's terms on the download page: following them is up to you.
- Get the Missing Fonts
- Gray-25%
- Gray-50%
- Ignore Once
- Indent Leading Spaces
- Indents to Leading Spaces
- Insert Blank Paragraphs Between
- Install Language Pack
- Inverse
- Item Color
- Join Lines
- Language Packs
- Language data
- Language packs aren't available in the browser
- Markdown
- Markdown Source
- Markdown Source Text
- Missing Fonts
- Mouse wheel scroll speed:
- OpenDocument Text Template (*.ott)
- Or install it for every app: double-click each font file and choose Install in Font Book, then restart WordCraft.
- Paragraph\nTools
- Paragraph Tools
- Pick the font files:
- Reading Page Color
- Reject All Changes and Stop Tracking
- Reject and Move to Next
- Remove Extra Spaces
- Remove Extra Spaces in the Whole Document
- Remove Language Pack
- Remove Leading Spaces
- Remove Leading Spaces in the Whole Document
- Remove Line Breaks
- Scale Font Sizes
- Screen Paper Color
- Screen only. Document backgrounds, printing and exports keep their colours. Dark page and explicit Read Mode colours take precedence.
- Screen paper color:
- Scrolling
- Select Canvas Member
- Separate Paragraphs
- Separate text with
- Sepia
- Set the colors in the Text Pane to use custom colors.
- Shape color
- Shown as {fonts} · {publisher}
- Shown in substitute fonts. Click for details.
- Special:
- Split Lines into Paragraphs
- Status bar
- Status bar count:
- Strict Export Report
- Styles Pane Position
- Switch\nWindows
- Switch Windows
- Synchronous Scrolling
- Teal
- Tell me when a document's missing fonts can be downloaded
- Text in a missing font is shown in another font, so it may not look as it was written. The downloads are free of charge.
- Then add it to WordCraft
- Touchpad scroll speed:
- Turquoise
- Update automatically
- View Side by Side
- You can also right-click the count in the status bar to change it.
- a shape inside a drawing canvas is placed by the canvas
- not in Read Mode
- not in a protected document
- not with tracked changes
- only floating connectors in the body text can be glued
- only for Markdown (.md) files
- only in Markdown Source
- only the desktop app has several windows
- turn on View Side by Side first
- {font} isn't installed on this computer, so its text is shown in {shown}. Line breaks and page count can differ from where the document was made. If you have the font file, add it with Add Font… at the end of this list.

La fenêtre de mise à jour au démarrage, qui restait en anglais, n'apparaît plus (commit
`1fdc4cdd`).

Règle : une modification ne doit pas allonger cette liste ; un nouveau texte d'interface
reçoit sa traduction française (dans craft-i18n, ou dans un catalogue propre à WriteCraft).
