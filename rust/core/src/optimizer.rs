use crate::model::{Catalogus, Profiel, ProjectLijn};

const FFD_LIMIET: usize = 40;

#[derive(Debug, Clone)]
pub struct Bak {
    pub lengte: i64,
    pub stukken: Vec<i64>,
    pub rest: i64,
}

fn past_in_bak(lengte: i64, bak: &Bak, zaagsnede: i64) -> bool {
    // Een deel dat exact de resterende lengte heeft, hoeft niet gezaagd te worden (geen zaagsnede).
    // Anders kost elk zaagdeel lengte + zaagsnede (om 2000 mm tot 1000 mm te reduceren verlies je 3 mm).
    if lengte == bak.rest {
        return true;
    }
    lengte + zaagsnede <= bak.rest
}

/// Hoeveel van de bak een stuk verbruikt (lengte + zaagsnede, behalve bij exacte pasvorm).
fn verbruik(lengte: i64, bak: &Bak, zaagsnede: i64) -> i64 {
    if lengte == bak.rest {
        lengte
    } else {
        lengte + zaagsnede
    }
}

pub fn pack_ffd(lengtes: &[i64], stock_lengte: i64, zaagsnede: i64) -> Vec<Bak> {
    let mut items: Vec<i64> = lengtes.to_vec();
    items.sort_by(|a, b| b.cmp(a));
    let mut bakken: Vec<Bak> = Vec::new();
    for item in items {
        let mut geplaatst = false;
        for bak in bakken.iter_mut() {
            if past_in_bak(item, bak, zaagsnede) {
                let v = verbruik(item, bak, zaagsnede);
                bak.stukken.push(item);
                bak.rest -= v;
                geplaatst = true;
                break;
            }
        }
        if !geplaatst {
            let v = if item == stock_lengte { item } else { item + zaagsnede };
            bakken.push(Bak {
                lengte: stock_lengte,
                stukken: vec![item],
                rest: stock_lengte - v,
            });
        }
    }
    bakken
}

/// Optimaal (minimaal aantal bakken) verpakken; None als een deel te lang is.
pub fn pack_optimaal(lengtes: &[i64], stock_lengte: i64, zaagsnede: i64) -> Option<Vec<Bak>> {
    let mut items: Vec<i64> = lengtes.to_vec();
    items.sort_by(|a, b| b.cmp(a));
    if items.is_empty() {
        return Some(Vec::new());
    }
    if items.len() > FFD_LIMIET {
        return Some(pack_ffd(lengtes, stock_lengte, zaagsnede));
    }
    if items.iter().any(|&l| l > stock_lengte) {
        return None;
    }

    struct Staat {
        bakken: Vec<Bak>,
        beste: Option<Vec<Bak>>,
    }

    fn rec(staat: &mut Staat, items: &[i64], i: usize, stock_lengte: i64, zaagsnede: i64) {
        if let Some(beste) = &staat.beste {
            if staat.bakken.len() >= beste.len() {
                return;
            }
        }
        if i == items.len() {
            if staat.beste.as_ref().map_or(true, |b| staat.bakken.len() < b.len()) {
                staat.beste = Some(staat.bakken.clone());
            }
            return;
        }
        let mut geprobeerd: Vec<(i64, usize)> = Vec::new();
        for j in 0..staat.bakken.len() {
            let sleutel = (staat.bakken[j].rest, staat.bakken[j].stukken.len());
            if geprobeerd.contains(&sleutel) {
                continue;
            }
            if past_in_bak(items[i], &staat.bakken[j], zaagsnede) {
                geprobeerd.push(sleutel);
                let v = verbruik(items[i], &staat.bakken[j], zaagsnede);
                staat.bakken[j].stukken.push(items[i]);
                staat.bakken[j].rest -= v;
                rec(staat, items, i + 1, stock_lengte, zaagsnede);
                staat.bakken[j].rest += v;
                staat.bakken[j].stukken.pop();
            }
        }
        let v = if items[i] == stock_lengte { items[i] } else { items[i] + zaagsnede };
        staat.bakken.push(Bak {
            lengte: stock_lengte,
            stukken: vec![items[i]],
            rest: stock_lengte - v,
        });
        rec(staat, items, i + 1, stock_lengte, zaagsnede);
        staat.bakken.pop();
    }

    let mut staat = Staat {
        bakken: Vec::new(),
        beste: None,
    };
    rec(&mut staat, &items, 0, stock_lengte, zaagsnede);
    staat.beste
}

#[derive(Debug, Clone)]
pub struct ProfielResultaat {
    /// (lengte_mm, prijs_per_stuk, aantal, bedrag) per ingekochte lengte
    pub per_lengte: Vec<(i64, f64, usize, f64)>,
    pub aantal: usize,
    pub bakken: Vec<Bak>,
    pub kosten: f64,
    pub afval: i64,
}

fn maak_resultaat(bakken: Vec<Bak>, inkoop: &[(i64, f64)]) -> ProfielResultaat {
    let prijs_per_lengte = |l: i64| inkoop.iter().find(|(sl, _)| *sl == l).map(|(_, p)| *p).unwrap_or(0.0);
    let mut per_lengte: Vec<(i64, f64, usize, f64)> = Vec::new();
    for bak in &bakken {
        if let Some(e) = per_lengte.iter_mut().find(|(l, _, _, _)| *l == bak.lengte) {
            e.2 += 1;
            e.3 = e.2 as f64 * e.1;
        } else {
            let p = prijs_per_lengte(bak.lengte);
            per_lengte.push((bak.lengte, p, 1, p));
        }
    }
    let kosten: f64 = per_lengte.iter().map(|(_, _, _, b)| *b).sum();
    let afval = bakken.iter().map(|b| b.rest).sum();
    ProfielResultaat {
        per_lengte,
        aantal: bakken.len(),
        bakken,
        kosten,
        afval,
    }
}

/// Kies de goedkoopste samenstelling van inkooplengtes (gemengd mogelijk) voor een set zaagdelen.
pub fn optimaliseer_profiel(
    lengtes: &[i64],
    inkoop: &[(i64, f64)],
    zaagsnede: i64,
) -> Option<ProfielResultaat> {
    if inkoop.is_empty() {
        return None;
    }
    let mut items: Vec<i64> = lengtes.to_vec();
    items.sort_by(|a, b| b.cmp(a));
    if items
        .iter()
        .any(|&l| !inkoop.iter().any(|&(sl, _)| l <= sl))
    {
        return None;
    }
    if items.len() > FFD_LIMIET {
        // fallback voor grote aantallen: alleen uniforme lengtes evalueren
        let mut beste: Option<ProfielResultaat> = None;
        for &(l, p) in inkoop {
            let Some(bakken) = pack_optimaal(lengtes, l, zaagsnede) else {
                continue;
            };
            let kosten = bakken.len() as f64 * p;
            let afval: i64 = bakken.iter().map(|b| b.rest).sum();
            let is_beter = match &beste {
                None => true,
                Some(b) => {
                    kosten < b.kosten - 1e-9
                        || ((kosten - b.kosten).abs() <= 1e-9 && afval < b.afval)
                }
            };
            if is_beter {
                beste = Some(ProfielResultaat {
                    per_lengte: vec![(l, p, bakken.len(), kosten)],
                    aantal: bakken.len(),
                    bakken,
                    kosten,
                    afval,
                });
            }
        }
        return beste;
    }

    // exacte branch & bound over alle inkooplengtes (gemengde samenstelling mogelijk)
    #[derive(Clone)]
    struct Bin {
        lengte: i64,
        rest: i64,
        stukken: Vec<i64>,
    }

    fn rec(
        bins: &mut Vec<Bin>,
        items: &[i64],
        i: usize,
        inkoop: &[(i64, f64)],
        zaagsnede: i64,
        kosten: f64,
        best: &mut Option<(f64, Vec<Bin>)>,
    ) {
        if let Some((bc, _)) = best {
            if kosten >= *bc - 1e-9 {
                return;
            }
        }
        if i == items.len() {
            if best.as_ref().map_or(true, |(bc, _)| kosten < *bc - 1e-9) {
                *best = Some((kosten, bins.clone()));
            }
            return;
        }
        let mut geprobeerd: Vec<(i64, i64)> = Vec::new();
        for j in 0..bins.len() {
            let sleutel = (bins[j].rest, bins[j].lengte);
            if geprobeerd.contains(&sleutel) {
                continue;
            }
            if items[i] == bins[j].rest || items[i] + zaagsnede <= bins[j].rest {
                geprobeerd.push(sleutel);
                let v = if items[i] == bins[j].rest {
                    items[i]
                } else {
                    items[i] + zaagsnede
                };
                bins[j].stukken.push(items[i]);
                bins[j].rest -= v;
                rec(bins, items, i + 1, inkoop, zaagsnede, kosten, best);
                bins[j].rest += v;
                bins[j].stukken.pop();
            }
        }
        for &(l, p) in inkoop {
            if items[i] <= l {
                let v = if items[i] == l { items[i] } else { items[i] + zaagsnede };
                bins.push(Bin {
                    lengte: l,
                    rest: l - v,
                    stukken: vec![items[i]],
                });
                rec(bins, items, i + 1, inkoop, zaagsnede, kosten + p, best);
                bins.pop();
            }
        }
    }

    let mut bins: Vec<Bin> = Vec::new();
    let mut best: Option<(f64, Vec<Bin>)> = None;
    rec(&mut bins, &items, 0, inkoop, zaagsnede, 0.0, &mut best);
    best.map(|(_, bins)| {
        let bakken = bins
            .into_iter()
            .map(|b| Bak {
                lengte: b.lengte,
                stukken: b.stukken,
                rest: b.rest,
            })
            .collect();
        maak_resultaat(bakken, inkoop)
    })
}

pub fn rond2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[derive(Debug, Clone)]
pub struct Berekening {
    pub zaagsnede_mm: i64,
    pub per_stuk: Vec<PerStukKosten>,
    pub zaag_resultaten: Vec<ZaagResultaat>,
    pub totaal_excl: f64,
}

#[derive(Debug, Clone)]
pub struct PerStukKosten {
    pub naam: String,
    /// Benodigd aantal stuks.
    pub aantal: u32,
    /// Inkoopeenheid (bijv. "doos"); leeg als per stuk ingekocht.
    pub eenheid: String,
    pub aantal_per_eenheid: f64,
    /// Aantal inkoopeenheden dat aangeschaft moet worden.
    pub inkoop_aantal: u32,
    pub prijs_per_eenheid: f64,
    pub prijs_per_stuk: f64,
    pub kosten: f64,
}

#[derive(Debug, Clone)]
pub struct ZaagResultaat {
    pub profiel: Profiel,
    pub lengtes: Vec<i64>,
    pub resultaat: Option<ProfielResultaat>,
    pub kosten: f64,
}

pub fn bereken_project(catalogus: &Catalogus, project: &[ProjectLijn]) -> Berekening {
    let zaagsnede = catalogus.instellingen.zaagsnede_mm.round() as i64;

    let mut per_stuk: Vec<PerStukKosten> = Vec::new();
    let mut zaag_groepen: Vec<(String, Vec<i64>)> = Vec::new();

    for lijn in project {
        match lijn {
            ProjectLijn::PerStuk {
                materiaal_id,
                aantal,
                ..
            } => {
                if let Some(m) = materiaal_id
                    .as_ref()
                    .and_then(|id| catalogus.per_stuk.iter().find(|m| &m.id == id))
                {
                    let stukprijs = m.stukprijs();
                    let inkoop_aantal = if m.aantal_per_eenheid > 0.0 {
                        ((*aantal as f64) / m.aantal_per_eenheid).ceil() as u32
                    } else {
                        *aantal
                    };
                    let kosten = rond2(*aantal as f64 * stukprijs);
                    per_stuk.push(PerStukKosten {
                        naam: m.naam.clone(),
                        aantal: *aantal,
                        eenheid: m.eenheid.clone(),
                        aantal_per_eenheid: m.aantal_per_eenheid,
                        inkoop_aantal,
                        prijs_per_eenheid: m.prijs_per_eenheid,
                        prijs_per_stuk: stukprijs,
                        kosten,
                    });
                }
            }
            ProjectLijn::Zaagdeel {
                profiel_id,
                lengte_mm,
                aantal,
                ..
            } => {
                let Some(pid) = profiel_id else { continue };
                if let Some(groep) = zaag_groepen.iter_mut().find(|(id, _)| id == pid) {
                    for _ in 0..*aantal {
                        groep.1.push(*lengte_mm);
                    }
                } else {
                    zaag_groepen.push((pid.clone(), vec![*lengte_mm; *aantal as usize]));
                }
            }
        }
    }

    let mut zaag_resultaten: Vec<ZaagResultaat> = Vec::new();
    let mut totaal = 0.0;
    for (pid, lengtes) in zaag_groepen {
        let Some(profiel) = catalogus.profielen.iter().find(|p| p.id == pid).cloned() else {
            continue;
        };
        let inkoop: Vec<(i64, f64)> = profiel
            .inkoop
            .iter()
            .map(|o| (o.lengte_mm, o.prijs))
            .collect();
        let resultaat = optimaliseer_profiel(&lengtes, &inkoop, zaagsnede);
        let kosten = rond2(resultaat.as_ref().map_or(0.0, |r| r.kosten));
        totaal += kosten;
        zaag_resultaten.push(ZaagResultaat {
            profiel,
            lengtes,
            resultaat,
            kosten,
        });
    }
    for l in &per_stuk {
        totaal += l.kosten;
    }

    Berekening {
        zaagsnede_mm: zaagsnede,
        per_stuk,
        zaag_resultaten,
        totaal_excl: rond2(totaal),
    }
}
