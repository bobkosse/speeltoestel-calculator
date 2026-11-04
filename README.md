# Speeltoestel Calculator — Rust (egui)

Native desktop-app in Rust met [egui](https://egui.rs), compileert tot één executable (~20 MB)
of een macOS .app-bundel.

De zaag- en kostenlogica zit in de aparte crate `core` (`speeltoestel-core`), los van de UI.

## Werking

De navigatie zit in de **linker zijbalk** (Projecten · Materialen · Project · Calculatie).

- **Projecten** — overzicht van alle projecten in één bestand (`projecten.json`): per project
  zie je het aantal onderdelen en het berekende totaal excl. BTW. Hier voeg je projecten toe,
  hernoem je ze, open je ze (om te bewerken in het tabblad Project) of verwijder je ze.
- **Materialen** — de gedeelde materialenbibliotheek: producten per stuk (schroeven, bouten:
  prijs/stuk + extra info) en half-fabricaten (profielen met houtsoort, dikte, breedte en één
  of meer inkooplengtes met prijs). Instellingen: zaagsnede in mm (standaard 3). Alle prijzen zijn exclusief BTW.
  Wordt opgeslagen als `materialen.json` — één bestand voor alle projecten.
- **Project** — de benodigde onderdelen van het geopende project: per stuk (materiaal + aantal)
  of zaagdeel (profiel + lengte in mm + breedte in mm + aantal).
- **Calculatie** — automatisch de goedkoopste inkoop (gemengde lengtes mogelijk), mét zaagsnede:
  een gezaagd deel kost lengte + zaagsnede (een stuk van 1000 mm uit een 2000 mm plank laat 997 mm
  rest; een deel met exact de planklengte kost geen zaagsnede). Een plank kan ook over de lengte
  geript worden voor smallere breedtes. Toont een Bill of Materials (aanschaf + prijzen), een
  zaagplan per plank en is direct af te drukken als PDF (met printerkeuze) of op te slaan als PDF.
- **Fool-proof opslaan** — bij de eerste start kies je een **werkmap**. Daarin staan
  `materialen.json` en `projecten.json`. De werkmap wordt onthouden (per gebruiker, in de
  app-instellingen) en bij elke start automatisch geladen. **Elke wijziging wordt direct
  automatisch weggeschreven** en bij het afsluiten van de app nogmaals geforceerd opgeslagen.
  Met *Werkmap wijzigen…* kun je de map achteraf aanpassen.

## Bouwen en draaien

Vereist: [Rust](https://rustup.rs) (stable).

```bash
cargo run --release        # direct draaien
cargo build --release      # executable staat in target/release/speeltoestel-calculator
```

Op macOS kun je de binary direct als app gebruiken; met `cargo install --path app` installeer je hem in `~/.cargo/bin`.

In de PDF/print staat onderaan elke pagina de footer: “Afgedrukt met Speelgoed Calculator van Robuust Buitenspelen”.

### macOS .app-bundel (aanbevolen — geen terminal bij dubbelklikken in Finder)

```bash
./make_app_bundle.sh
```

Dit bouwt `target/release/Speeltoestel Calculator.app` (en een `.dmg`). Dubbelklik de .app in Finder: alleen de GUI opent.

**Installeren in je Apps-map (/Applications):**

```bash
./make_app_bundle.sh --install
```

Dan staat de app vast in `/Applications` (bij fehlende rechten krijg je de macOS admin-dialoog). Het favicon van www.robuustbuitenspelen.nl zit als icoon in `app/assets/AppIcon.iconset/` en wordt automatisch als `AppIcon.icns` in de .app meegenomen. Bij Gatekeeper-melding: rechtermuisknop → Open.

## Testen

```bash
cargo test
```

## Structuur

```
core/    datamodel (serde) + zaagoptimalisatie + tests
app/     egui-interface (main.rs) + assets (icoon, fonts)
```

Bestandsformaten: `Catalogus` (materialenbibliotheek) en `ProjectData` (project), zie `core/src/model.rs`.
