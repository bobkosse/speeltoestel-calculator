use crate::model::{Catalogus, Profiel, ProjectLijn};

const FFD_LIMIET: usize = 40;

#[derive(Debug, Clone)]
pub struct Bak {
    pub stukken: Vec<i64>,
    pub rest: i64,
}

fn past_in_bak(lengte: i64, bak: &Bak, zaagsnede: i64) -> bool {
    // Elk zaagdeel kost lengte + zaagsnede uit de plank (ook het eerste stuk:
    // om een plank van 2000 mm tot 1000 mm te reduceren verlies je 3 mm aan de zaagsnede).
    lengte + zaagsnede <= bak.rest
}

pub fn pack_ffd(lengtes: &[i64], stock_lengte: i64, zaagsnede: i64) -> Vec<Bak> {
    let mut items: Vec<i64> = lengtes.to_vec();
    items.sort_by(|a, b| b.cmp(a));
    let mut bakken: Vec<Bak> = Vec::new();
    for item in items {
        let mut geplaatst = false;
        for bak in bakken.iter_mut() {
            if past_in_bak(item, bak, zaagsnede) {
                bak.stukken.push(item);
                bak.rest -= item + zaagsnede;
                geplaatst = true;
                break;
            }
        }
        if !geplaatst {
            bakken.push(Bak {
                stukken: vec![item],
                rest: stock_lengte - item - zaagsnede,
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
                staat.bakken[j].stukken.push(items[i]);
                staat.bakken[j].rest -= items[i] + zaagsnede;
                rec(staat, items, i + 1, stock_lengte, zaagsnede);
                staat.bakken[j].rest += items[i] + zaagsnede;
                staat.bakken[j].stukken.pop();
            }
        }
        staat.bakken.push(Bak {
            stukken: vec![items[i]],
            rest: stock_lengte - items[i] - zaagsnede,
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
    pub lengte_mm: i64,
    pub prijs_per_stuk: f64,
    pub aantal: usize,
    pub bakken: Vec<Bak>,
    pub kosten: f64,
    pub afval: i64,
}

/// Kies de goedkoopste inkooplengte voor een set zaagdelen.
pub fn optimaliseer_profiel(
    lengtes: &[i64],
    inkoop: &[(i64, f64)],
    zaagsnede: i64,
) -> Option<ProfielResultaat> {
    let mut beste: Option<ProfielResultaat> = None;
    for &(lengte_mm, prijs) in inkoop {
        let Some(bakken) = pack_optimaal(lengtes, lengte_mm, zaagsnede) else {
            continue;
        };
        let kosten = bakken.len() as f64 * prijs;
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
                lengte_mm,
                prijs_per_stuk: prijs,
                aantal: bakken.len(),
                bakken,
                kosten,
                afval,
            });
        }
    }
    beste
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
    pub aantal: u32,
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
                    per_stuk.push(PerStukKosten {
                        naam: m.naam.clone(),
                        aantal: *aantal,
                        prijs_per_stuk: m.prijs,
                        kosten: rond2(*aantal as f64 * m.prijs),
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
