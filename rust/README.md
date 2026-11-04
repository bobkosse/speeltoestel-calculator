# Speeltoestel Calculator — Rust (egui)

Native desktop-app in Rust met [egui](https://egui.rs). Wird als **enkele executable** gecompileerd
(ca. 20 MB, tegen ca. 200 MB voor de Electron-versie in de root van deze repo).

De zaag- en kostenlogica zit in de aparte crate `core` (`speeltoestel-core`), los van de UI.
Het JSON-bestandsformaat is identiek aan de Electron-versie, dus bestanden zijn uitwisselbaar.

## Werking

De navigatie zit in de **linker zijbalk** (Projecten · Materialen · Project · Calculatie).

- **Projecten** — overzicht van alle projecten in één bestand (`projecten.json`): per project
  zie je het aantal onderdelen en het berekende totaal excl. BTW. Hier voeg je projecten toe,
  hernoem je ze, open je ze (om te bewerken in het tabblad Project) of verwijder je ze.
- **Materialen** — de gedeelde materialenbibliotheek: producten per stuk (schroeven, bouten:
  prijs/stuk + extra info) en half-fabricaten (profielen met houtsoort, dikte, breedte en één
  of meer inkooplengtes met prijs). Instellingen: zaagsnede in mm (standaard 3). Alle prijzen zijn exclusief BTW.
  Sla op via *Bibliotheek opslaan* als `materialen.json` — één bestand voor alle projecten.
- **Project** — de benodigde onderdelen van het geopende project: per stuk (materiaal + aantal)
  of zaagdeel (profiel + lengte in mm + aantal).
- **Calculatie** — automatisch de goedkoopste inkoop, mét zaagsnede: n delen uit één plank
  kosten `som(lengtes) + (n−1) × zaagsnede` (2 × 1000 mm past dus niet in een 2 m plank).
  De calculatie toont een Bill of Materials (wat je aanschaft, met prijs per stuk en bedrag) plus een overzicht van reststukken ter controle. Zaagsnede wordt per zaagdeel meegerekend: een stuk van 1000 mm uit een 2000 mm plank kost 1003 mm, reststuk 997 mm.
- **Fool-proof opslaan** — bij de eerste start kies je een **werkmap**. Daarin staan
  `materialen.json` en `projecten.json`. De werkmap wordt onthouden (per gebruiker, in de
  app-instellingen) en bij elke start automatisch geladen. **Elke wijziging wordt direct
  automatisch weggeschreven** en bij het afsluiten van de app nogmaals geforceerd opgeslagen.
  Met *Werkmap wijzigen…* kun je de map achteraf aanpassen.

## Bouwen en draaien

Vereist: [Rust](https://rustup.rs) (stable).

```bash
cd rust
cargo run --release        # direct draaien
cargo build --release      # executable staat in target/release/speeltoestel-calculator
```

Op macOS kun je de binary direct als app gebruiken; met `cargo install --path app` installeer je hem in `~/.cargo/bin`.

### macOS .app-bundel (aanbevolen — geen terminal bij dubbelklikken in Finder)

```bash
./make_app_bundle.sh
```

Dit bouwt `target/release/Speeltoestel Calculator.app` (en een `.dmg`). Dubbelklik de .app in Finder: alleen de GUI opent. Eventueel icoon: plaats PNG's in `app/assets/AppIcon.iconset/` (16–512 px), dan wordt automatisch `AppIcon.icns` meegenomen. Bij Gatekeeper-melding: rechtermuisknop → Open.

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
