# Speeltoestel Calculator — Rust (egui)

Native desktop-app in Rust met [egui](https://egui.rs). Wird als **enkele executable** gecompileerd
(ca. 20 MB, tegen ca. 200 MB voor de Electron-versie in de root van deze repo).

De zaag- en kostenlogica zit in de aparte crate `core` (`speeltoestel-core`), los van de UI.
Het JSON-bestandsformaat is identiek aan de Electron-versie, dus bestanden zijn uitwisselbaar.

## Werking

- **Materialen** — producten per stuk (schroeven, bouten: prijs/stuk + extra info) en
  half-fabricaten (profielen met houtsoort, dikte, breedte en één of meer inkooplengtes met prijs).
  Instellingen: zaagsnede in mm (standaard 3) en BTW-%.
- **Project** — benodigde onderdelen: per stuk (materiaal + aantal) of zaagdeel
  (profiel + lengte in mm + aantal).
- **Calculatie** — automatisch de goedkoopste inkoop, mét zaagsnede: n delen uit één plank
  kosten `som(lengtes) + (n−1) × zaagsnede` (2 × 1000 mm past dus niet in een 2 m plank).
  Per profiel zie je het zaagplan en het restafval. Prijzen excl. BTW, plus totaal incl. BTW.
- **Bestanden (gesplitst)** —
  - *Materialenbibliotheek* (`materialen.json`): instellingen, producten per stuk en profielen.
    Deze hou je **één keer** bij en deel je tussen alle projecten via Openen…/Opslaan.
  - *Project* (`speeltoestel-project.json`): alleen de benodigde onderdelen. Per project
    Nieuw/Openen…/Opslaan/Opslaan als…. Oude bestanden met materialen + project in één bestand
    worden automatisch gesplitst bij het openen.

## Bouwen en draaien

Vereist: [Rust](https://rustup.rs) (stable).

```bash
cd rust
cargo run --release        # direct draaien
cargo build --release      # executable staat in target/release/speeltoestel-calculator
```

Op macOS kun je de binary direct als app gebruiken; met `cargo install --path app` installeer je hem in `~/.cargo/bin`.

## Testen

```bash
cd rust
cargo test
```

## Structuur

```
rust/
  core/    datamodel (serde) + zaagoptimalisatie + tests
  app/     egui-interface (main.rs)
```

Bestandsformaten: `Catalogus` (materialenbibliotheek) en `ProjectData` (project), zie `core/src/model.rs`.
