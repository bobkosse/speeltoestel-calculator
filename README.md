# Speeltoestel Calculator

Desktop-app (Electron) voor het snel maken van kostprijscalculaties voor houten
speeltoestellen. Prijzen worden **exclusief BTW** ingevoerd; de app toont daarnaast
het totaal inclusief BTW.

## Werking

- **Materialen tab**
  - *Producten per stuk*: schroeven, bouten, etc. — prijs per stuk plus vrije extra info
    (bijv. diameter en lengte).
  - *Half-fabricaten*: profielen zoals planken (houtsoort, dikte, breedte) of palen.
    Per profiel geef je één of meer **inkooplengtes** op met prijs per stuk.
  - *Instellingen*: zaagsnede in mm (standaard 3 mm) en BTW-percentage.
- **Project tab**: voeg benodigde onderdelen toe — per stuk (materiaal + aantal) of
  zaagdeel (profiel + gewenste lengte in mm + aantal).
- **Calculatie tab**: de app berekent automatisch de **goedkoopste inkoop**.
  Dabei wordt rekening gehouden met de zaagsnede: voor n delen uit één plank zijn
  n−1 zaagsnedes nodig (2 × 100 mm kost dus 203 mm, geen 200 mm). Als je per profiel
  meerdere lengtes hebt opgegeven (bijv. 1000 mm en 2000 mm), kiest de app de
  voordeligste samenstelling. Per profiel zie je ook het zaagplan: welke delen uit
  welke ingekochte plank komen en hoeveel restafval dat oplevert.

## Installeren en starten

Vereist: [Node.js](https://nodejs.org) (versie 18 of hoger).

```bash
npm install
npm start
```

## Testen

De zaag- en kostenlogica heeft unit tests:

```bash
npm test
```

## Troubleshooting

**`Error: Electron failed to install correctly` na `npm start`**

Het Electron-binary is dan niet (volledig) gedownload. Los dit op met:

```bash
rm -rf node_modules/electron
npm install
node node_modules/electron/install.js
```

Controleer daarna:

```bash
cat node_modules/electron/path.txt        # moet "dist/Electron.app" (macOS) bevatten
ls node_modules/electron/dist/
```

Op macOS kan Gatekeeper het binary blokkeren; verwijder de quarantine-flag:

```bash
xattr -dr com.apple.quarantine node_modules/electron/dist/Electron.app
```

Daarna opnieuw:

```bash
npm start
```

Werkt het nog steeds niet (bijv. achter een strikte proxy)? Installeer met scripts aan:

```bash
npm install --include=dev
```

## Opslag

Projecten worden opgeslagen als **JSON-bestand** via Bestand → Openen… / Opslaan /
Opslaan als… (ook met de sneltoetsen Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S).

## Bestandsformaat (JSON)

```json
{
  "versie": 1,
  "instellingen": { "zaagsnede_mm": 3, "btw_pct": 21, "munt": "EUR" },
  "per_stuk": [
    { "id": "s1", "naam": "Schroef RVS 5 x 60 mm", "prijs": 0.12, "info": "diameter 5 mm, lengte 60 mm" }
  ],
  "profielen": [
    {
      "id": "p1", "houtsoort": "Robinia", "dikte_mm": 45, "breedte_mm": 120,
      "inkoop": [{ "lengte_mm": 2000, "prijs": 48.50 }]
    }
  ],
  "project": [
    { "id": "l1", "type": "zaagdeel", "profiel_id": "p1", "lengte_mm": 1000, "aantal": 2 },
    { "id": "l2", "type": "per_stuk", "materiaal_id": "s1", "aantal": 24 }
  ]
}
```

`type` in `project` is `"per_stuk"` of `"zaagdeel"`.
