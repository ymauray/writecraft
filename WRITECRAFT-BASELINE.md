# WriteCraft — base de référence des tests

> Relevé le 2026-10-31 sur le Mac (macOS 27.0.1, build 26A434), commit `69e5ec0b`,
> cargo-nextest 0.9.150. Le code n'a pas été modifié : ce relevé décrit l'état de départ.
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

| Étape | Résultat |
|---|---|
| Tests (nextest, `--no-fail-fast`) | 4590 lancés : **4588 réussis, 2 échoués, 5 ignorés** (46 s) |
| Audit i18n (`i18n-report`) | ✅ réussi (103 s) ; rapport des traductions manquantes non produit, faute de `CRAFT_I18N_DIR` |
| Doctests | ✅ aucun échec |
| fmt, clippy, assets, layers, wasm | pas relancés dans ce relevé |

## Échecs connus (2)

| Test | Message | Cause probable |
|---|---|---|
| `wordcraft-fonts fontdb::tests::a_thai_run_with_latin_falls_back_to_one_face` | `Thonburi shapes Thai` (`crates/fonts/src/fontdb/tests.rs:115`) | la police système Thonburi de macOS 27 n'a pas de table `GSUB` (`has_layout_tables`) ; dépend des polices installées |
| `wordcraft-ui-egui canvas::tests::a_right_to_left_balloon_reads_from_the_right` | `wrapped glyph 'ش': origin 358, max 44.341774, row width 41.9375, limit 400` (`crates/ui-egui/src/canvas/tests.rs:1157`) | rendu du texte arabe avec les polices du Mac ; non analysé |

Ces deux tests dépendent des polices système : ils peuvent passer sur une autre machine.

## Tests ignorés (5)

| Test | Raison |
|---|---|
| `wordcraft-engine::it recovery_format::native_json_size_with_a_large_picture` | `#[ignore = "measurement"]` |
| `wordcraft-fonts fontdb::tests::kai_contours_include_complete_character_body` | nécessite `kaiu.ttf` sous licence (`WORDCRAFT_TEST_KAI_FONT`) |
| `wordcraft-ui-egui mini_toolbar::tests::the_bar_scrolls_under_the_ribbon_not_over_it` | en conflit avec l'ancre de sélection de #1501 |
| `wordcraft-ui-egui tests::perf::big_table_scroll_bench` | banc de performance (`#[ignore]`) |
| `wordcraft-ui-egui i18n::audit::untranslated_interface_text` | exclu de la passe principale, lancé à part par `cargo xtask i18n-report` (réussi) |
