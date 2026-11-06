pub mod model;
pub mod optimizer;

#[cfg(test)]
mod tests {
    use super::model::*;
    use super::optimizer::*;

    #[test]
    fn zaagsnede_1x1000_uit_2000_geeft_rest_997() {
        let bakken = pack_optimaal(&[1000], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 997);
    }

    #[test]
    fn zaagsnede_2x100_past_in_2000() {
        let bakken = pack_optimaal(&[100, 100], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 2000 - 100 - 3 - 100 - 3);
    }

    #[test]
    fn zaagsnede_2x1000_past_niet_in_2000() {
        let bakken = pack_optimaal(&[1000, 1000], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 2);
    }

    #[test]
    fn zaagsnede_3x500_past_in_2000() {
        let bakken = pack_optimaal(&[500, 500, 500], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 2000 - 1500 - 3 * 3);
    }

    #[test]
    fn exacte_lengte_past_zonder_zaagsnede() {
        // nodig 2000 mm, plank 2000 mm: exacte pasvorm, geen zaagsnede, rest 0
        let bakken = pack_optimaal(&[2000], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 0);
        // en in de inkoopkeuze wint 2000 mm boven 2500 mm
        let res = optimaliseer_profiel(&[Stuk { lengte: 2000, breedte: 120 }], 120, &[(2000, 48.50), (2500, 55.0)], 3).unwrap();
        assert_eq!(res.per_lengte, vec![(2000, 48.50, 1, 48.50)]);
        assert_eq!(res.kosten, 48.50);
    }

    #[test]
    fn plank_geript_over_lengte() {
        // 2 × (2000 x 58) uit een plank 2000 x 120: 58 + 3 + 58 = 119 <= 120 → 1 plank
        let res = optimaliseer_profiel(
            &[Stuk { lengte: 2000, breedte: 58 }, Stuk { lengte: 2000, breedte: 58 }],
            120,
            &[(2000, 48.50), (2500, 55.0)],
            3,
        ).unwrap();
        assert_eq!(res.aantal, 1);
        assert_eq!(res.kosten, 48.50);
        // 3 × 58 past niet (3*58 + 2*3 = 180 > 120) → 2 planken
        let res2 = optimaliseer_profiel(
            &[Stuk { lengte: 2000, breedte: 58 }, Stuk { lengte: 2000, breedte: 58 }, Stuk { lengte: 2000, breedte: 58 }],
            120,
            &[(2000, 48.50)],
            3,
        ).unwrap();
        assert_eq!(res2.aantal, 2);
    }

    #[test]
    fn stress_veel_delen_blijft_snel() {
        // 35 gemengde zaagdelen: moet binnen een seconde geoptimaliseerd zijn (node limit)
        let mut items: Vec<Stuk> = Vec::new();
        let lengtes = [2100, 1000, 950, 800, 730, 600, 500, 450, 400, 350, 300, 250, 200, 150, 100];
        for i in 0..35 {
            items.push(Stuk { lengte: lengtes[i % lengtes.len()], breedte: if i % 3 == 0 { 58 } else { 120 } });
        }
        let start = std::time::Instant::now();
        let res = optimaliseer_profiel(&items, 120, &[(2000, 48.50), (2500, 55.0)], 3).unwrap();
        let elapsed = start.elapsed();
        println!("35 delen: {} planken, €{:.2}, {:?}", res.aantal, res.kosten, elapsed);
        assert!(elapsed < std::time::Duration::from_secs(2), "te traag: {elapsed:?}");
    }

    #[test]
    fn zaagresultaten_gesorteerd_per_product() {
        let mut catalogus = test_catalogus();
        catalogus.profielen.push(Profiel {
            id: "p2".into(),
            houtsoort: "Vuren".into(),
            dikte_mm: 25,
            breedte_mm: 100,
            inkoop: vec![InkoopOptie { lengte_mm: 2000, prijs: 20.0 }],
        });
        let project = vec![
            ProjectLijn::Zaagdeel { id: "a".into(), profiel_id: Some("p1".into()), lengte_mm: 500, breedte_mm: None, aantal: 1 },
            ProjectLijn::Zaagdeel { id: "b".into(), profiel_id: Some("p2".into()), lengte_mm: 500, breedte_mm: None, aantal: 1 },
        ];
        let entry = ProjectEntry { id: "t".into(), naam: "t".into(), regels: project, groepen: Vec::new(), modules: Vec::new(), calc_hash: None, calc_totaal: None };
        let res = bereken_project(&catalogus, &entry);
        // p2 (25 mm) moet vóór p1 (45 mm)
        assert_eq!(res.zaag_resultaten[0].profiel.dikte_mm, 25);
        assert_eq!(res.zaag_resultaten[1].profiel.dikte_mm, 45);
    }

    #[test]
    fn module_calculatie_verkoopprijs_en_winst() {
        use crate::model::{Module, ProjectModule};
        let mut catalogus = test_catalogus();
        catalogus.instellingen.uur_tarief = 45.0;
        catalogus.per_stuk[0].marge_pct = 30.0;
        catalogus.modules = vec![Module {
            id: "m1".into(),
            naam: "Toren".into(),
            regels: vec![
                ProjectLijn::Zaagdeel { id: "a".into(), profiel_id: Some("p1".into()), lengte_mm: 1000, breedte_mm: None, aantal: 2 },
                ProjectLijn::PerStuk { id: "b".into(), materiaal_id: Some("s1".into()), aantal: 10 },
            ],
            uren_bouwen: 3.0,
            uren_plaatsen: 1.0,
            lengte_mm: 2500,
        }];
        let entry = ProjectEntry {
            id: "pr".into(),
            naam: "Knikkerbaan".into(),
            regels: Vec::new(),
            groepen: Vec::new(),
            modules: vec![ProjectModule { module_id: Some("m1".into()), aantal: 2 }],
            calc_hash: None,
            calc_totaal: None,
        };
        let res = bereken_project(&catalogus, &entry);
        // per module: hout 2×1000 uit 2000@40 = 2 planken = 80; schroeven 10×0,50 = 5 → inkoop 85
        // 2 modules → inkoop 170; marge = 2 × (5 × 30%) = 3; uren = 2 × 4 = 8; arbeid = 45×8 = 360
        assert_eq!(res.inkoop_totaal, 170.0);
        assert_eq!(res.marge_totaal, 3.0);
        assert_eq!(res.uren_totaal, 8.0);
        assert_eq!(res.arbeid, 360.0);
        assert_eq!(res.verkoopprijs, 533.0);
        assert_eq!(res.bruto_winst, 363.0);
        assert_eq!(res.modules.len(), 1);
        assert_eq!(res.modules[0].uren, 8.0);
        assert_eq!(res.modules[0].lengte_totaal, 5000);
        assert_eq!(res.lengte_totaal, 5000);
    }

    #[test]
    fn rechte_bak_voorbeeld_ripped_uit_1_plank() {
        // Module "Rechte bak": 2× (2000 x 58) + 20 schroeven à €0,0997
        // 2 stroken van 58 mm uit één 2000x120 plank: 58 + 3 + 58 = 119 <= 120 → 1 plank à €10,50
        let mut catalogus = test_catalogus();
        catalogus.per_stuk = vec![PerStuk {
            id: "s1".into(),
            naam: "RVS A4 4x50 Torx".into(),
            prijs: 0.0997,
            info: String::new(),
            eenheid: "stuk".into(),
            aantal_per_eenheid: 200.0,
            prijs_per_eenheid: 19.94,
            marge_pct: 300.0,
        }];
        catalogus.profielen = vec![Profiel {
            id: "p1".into(),
            houtsoort: "Robinia plank (25 mm dik)".into(),
            dikte_mm: 25,
            breedte_mm: 120,
            inkoop: vec![
                InkoopOptie { lengte_mm: 2000, prijs: 10.50 },
                InkoopOptie { lengte_mm: 2500, prijs: 13.13 },
                InkoopOptie { lengte_mm: 3000, prijs: 15.75 },
            ],
        }];
        let entry = ProjectEntry {
            id: "pr".into(),
            naam: "t".into(),
            regels: vec![
                ProjectLijn::Zaagdeel { id: "a".into(), profiel_id: Some("p1".into()), lengte_mm: 2000, breedte_mm: Some(58), aantal: 2 },
                ProjectLijn::PerStuk { id: "b".into(), materiaal_id: Some("s1".into()), aantal: 20 },
            ],
            groepen: Vec::new(),
            modules: Vec::new(),
            calc_hash: None,
            calc_totaal: None,
        };
        let res = bereken_project(&catalogus, &entry);
        // hout: 1 plank = €10,50; schroeven: 20 × €0,0997 = €1,99 → inkoop €12,49
        assert_eq!(res.zaag_resultaten[0].resultaat.as_ref().unwrap().aantal, 1);
        assert_eq!(res.inkoop_totaal, 12.49);
    }

    #[test]
    fn te_lang_deel_kan_niet() {
        assert!(pack_optimaal(&[2500], 2000, 3).is_none());
    }

    #[test]
    fn goedkoopste_lengte_2x900() {
        let res = optimaliseer_profiel(&[Stuk { lengte: 900, breedte: 120 }, Stuk { lengte: 900, breedte: 120 }], 120, &[(1000, 10.0), (2000, 25.0)], 3).unwrap();
        assert_eq!(res.per_lengte, vec![(1000, 10.0, 2, 20.0)]);
        assert_eq!(res.aantal, 2);
        assert_eq!(res.kosten, 20.0);
    }

    #[test]
    fn langere_plank_wint_als_delen_samen_passen() {
        let res = optimaliseer_profiel(&[Stuk { lengte: 900, breedte: 120 }, Stuk { lengte: 900, breedte: 120 }], 120, &[(1000, 12.0), (2000, 20.0)], 3).unwrap();
        assert_eq!(res.per_lengte, vec![(2000, 20.0, 1, 20.0)]);
        assert_eq!(res.aantal, 1);
        assert_eq!(res.kosten, 20.0);
    }

    #[test]
    fn zaagsnede_telt_mee_bij_lengtekeuze() {
        // 1000 mm stuk past niet in 1000 mm plank (1003 > 1000), 2100 mm wel (2006 <= 2100)
        let res = optimaliseer_profiel(&[Stuk { lengte: 1000, breedte: 120 }, Stuk { lengte: 1000, breedte: 120 }], 120, &[(1000, 12.0), (2100, 22.0)], 3).unwrap();
        assert_eq!(res.per_lengte, vec![(2100, 22.0, 1, 22.0)]);
        assert_eq!(res.kosten, 22.0);
    }

    #[test]
    fn user_voorbeeld_400_en_2000_goedkoopst_gemengd() {
        // nodig: 400 mm + 2000 mm; 1 x 2500 mm kost 60, maar 2100 mm (2000+zaagsnede) + 500 mm (400+zaagsnede) = 52
        let res = optimaliseer_profiel(&[Stuk { lengte: 400, breedte: 120 }, Stuk { lengte: 2000, breedte: 120 }], 120, &[(2500, 60.0), (2100, 40.0), (500, 12.0)], 3).unwrap();
        assert_eq!(res.kosten, 52.0);
        let mut per_lengte = res.per_lengte.clone();
        per_lengte.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(per_lengte, vec![(500, 12.0, 1, 12.0), (2100, 40.0, 1, 40.0)]);
    }

    #[test]
    fn gemengde_lengtes_winnen_als_dat_goedkoper_is() {
        // [2100, 1000, 1000, 1000] met 2000@48.50 en 2500@55, zaagsnede 3:
        // 3 x 2500 = 165,00, maar 2 x 2500 (2100; 1000+1000) + 1 x 2000 (1000) = 158,50
        let res = optimaliseer_profiel(&[Stuk { lengte: 2100, breedte: 120 }, Stuk { lengte: 1000, breedte: 120 }, Stuk { lengte: 1000, breedte: 120 }, Stuk { lengte: 1000, breedte: 120 }], 120, &[(2000, 48.50), (2500, 55.0)], 3).unwrap();
        assert_eq!(res.kosten, 158.50);
        assert_eq!(res.aantal, 3);
        let mut per_lengte = res.per_lengte.clone();
        per_lengte.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(per_lengte, vec![(2000, 48.50, 1, 48.50), (2500, 55.0, 2, 110.0)]);
    }

    #[test]
    fn lege_inkoop_is_geen_resultaat() {
        assert!(optimaliseer_profiel(&[Stuk { lengte: 100, breedte: 120 }], 120, &[], 3).is_none());
    }

    fn test_catalogus() -> Catalogus {
        Catalogus {
            versie: 1,
            instellingen: Instellingen {
                zaagsnede_mm: 3.0,
                munt: "EUR".to_string(),
                uur_tarief: 45.0,
                marge_hout_pct: 0.0,
            },
            per_stuk: vec![PerStuk {
                id: "s1".into(),
                naam: "Schroef 5x60".into(),
                prijs: 0.50,
                info: String::new(),
                eenheid: String::new(),
                aantal_per_eenheid: 0.0,
                prijs_per_eenheid: 0.0,
                marge_pct: 0.0,
            }],
            profielen: vec![Profiel {
                id: "p1".into(),
                houtsoort: "Robinia".into(),
                dikte_mm: 45,
                breedte_mm: 120,
                inkoop: vec![InkoopOptie {
                    lengte_mm: 2000,
                    prijs: 40.0,
                }],
            }],
            modules: Vec::new(),
        }
    }

    #[test]
    fn bereken_project_totaal() {
        let catalogus = test_catalogus();
        let project = vec![
            ProjectLijn::PerStuk {
                id: "l1".into(),
                materiaal_id: Some("s1".into()),
                aantal: 10,
            },
            ProjectLijn::Zaagdeel {
                id: "l2".into(),
                profiel_id: Some("p1".into()),
                lengte_mm: 1000,
                breedte_mm: None,
                aantal: 2,
            },
        ];
        let entry = ProjectEntry { id: "t".into(), naam: "t".into(), regels: project, groepen: Vec::new(), modules: Vec::new(), calc_hash: None, calc_totaal: None };
        let res = bereken_project(&catalogus, &entry);
        assert_eq!(res.zaag_resultaten[0].resultaat.as_ref().unwrap().aantal, 2);
        assert_eq!(res.zaag_resultaten[0].kosten, 80.0);
        assert_eq!(res.totaal_excl, 85.0);
    }

    #[test]
    fn per_stuk_met_verpakking_rekent_met_stukprijs() {
        // doos van 100 stuks à €12,00 → stukprijs €0,12; nodig 24 → kosten 24 × €0,12 = €2,88 (1 doos nodig)
        let mut catalogus = test_catalogus();
        catalogus.per_stuk = vec![PerStuk {
            id: "s1".into(),
            naam: "Schroef 5x60 (doos)".into(),
            prijs: 0.0,
            info: String::new(),
            eenheid: "doos".into(),
            aantal_per_eenheid: 100.0,
            prijs_per_eenheid: 12.0,
            marge_pct: 0.0,
        }];
        let project = vec![ProjectLijn::PerStuk {
            id: "l1".into(),
            materiaal_id: Some("s1".into()),
            aantal: 24,
        }];
        let entry = ProjectEntry { id: "t".into(), naam: "t".into(), regels: project, groepen: Vec::new(), modules: Vec::new(), calc_hash: None, calc_totaal: None };
        let res = bereken_project(&catalogus, &entry);
        let l = &res.per_stuk[0];
        assert!((l.prijs_per_stuk - 0.12).abs() < 1e-9);
        assert_eq!(l.aantal, 24);
        assert_eq!(l.inkoop_aantal, 1);
        assert_eq!(l.kosten, 2.88);
        assert_eq!(res.totaal_excl, 2.88);
    }

    #[test]
    fn bereken_project_deel_past_nergens() {
        let mut catalogus = Catalogus::default();
        catalogus.per_stuk.clear();
        catalogus.profielen = vec![Profiel {
            id: "p1".into(),
            houtsoort: "Vuren".into(),
            dikte_mm: 20,
            breedte_mm: 100,
            inkoop: vec![InkoopOptie {
                lengte_mm: 1000,
                prijs: 5.0,
            }],
        }];
        let project = vec![ProjectLijn::Zaagdeel {
            id: "l1".into(),
            profiel_id: Some("p1".into()),
            lengte_mm: 1500,
            breedte_mm: None,
            aantal: 1,
        }];
        let entry = ProjectEntry { id: "t".into(), naam: "t".into(), regels: project, groepen: Vec::new(), modules: Vec::new(), calc_hash: None, calc_totaal: None };
        let res = bereken_project(&catalogus, &entry);
        assert!(res.zaag_resultaten[0].resultaat.is_none());
        assert_eq!(res.totaal_excl, 0.0);
    }

    #[test]
    fn json_roundtrip_catalogus_en_project() {
        let catalogus = Catalogus::default();
        let json = serde_json::to_string_pretty(&catalogus).unwrap();
        let terug: Catalogus = serde_json::from_str(&json).unwrap();
        assert_eq!(terug.per_stuk.len(), 2);
        assert_eq!(terug.profielen.len(), 1);

        let projecten = ProjectenData {
            versie: 1,
            projecten: vec![ProjectEntry {
                id: "pr1".into(),
                naam: "Speeltoren".into(),
                calc_hash: None,
                calc_totaal: None,
                groepen: Vec::new(),
                modules: Vec::new(),
                regels: vec![
                    ProjectLijn::Zaagdeel {
                        id: "l1".into(),
                        profiel_id: Some("p1".into()),
                        lengte_mm: 1000,
                        breedte_mm: None,
                        aantal: 2,
                    },
                    ProjectLijn::PerStuk {
                        id: "l2".into(),
                        materiaal_id: Some("s1".into()),
                        aantal: 24,
                    },
                ],
            }],
        };
        let pjson = serde_json::to_string_pretty(&projecten).unwrap();
        let pterug: ProjectenData = serde_json::from_str(&pjson).unwrap();
        assert_eq!(pterug.projecten.len(), 1);
        assert_eq!(pterug.projecten[0].regels.len(), 2);
        let res = bereken_project(&terug, &pterug.projecten[0]);
        assert!(res.totaal_excl > 0.0);
    }

    #[test]
    fn legacy_gecombineerd_bestand_wordt_gesplitst() {
        // Oude bestanden bevatten catalogus + project in één JSON.
        let legacy = r#"{
            "versie": 1,
            "instellingen": { "zaagsnede_mm": 3, "btw_pct": 21, "munt": "EUR" },
            "per_stuk": [{ "id": "s1", "naam": "Schroef", "prijs": 0.5, "info": "" }],
            "profielen": [{ "id": "p1", "houtsoort": "Robinia", "dikte_mm": 45, "breedte_mm": 120,
                            "inkoop": [{ "lengte_mm": 2000, "prijs": 40 }] }],
            "project": [{ "id": "l1", "type": "zaagdeel", "profiel_id": "p1", "lengte_mm": 1000, "aantal": 2 }]
        }"#;
        let value: serde_json::Value = serde_json::from_str(legacy).unwrap();
        let catalogus: Catalogus = serde_json::from_value(value.clone()).unwrap();
        // legacy "project"-array wordt in de app verpakt in één ProjectEntry
        let regels: Vec<ProjectLijn> =
            serde_json::from_value(value["project"].clone()).unwrap();
        assert_eq!(catalogus.profielen.len(), 1);
        assert_eq!(regels.len(), 1);
        let entry = ProjectEntry { id: "t".into(), naam: "t".into(), regels, groepen: Vec::new(), modules: Vec::new(), calc_hash: None, calc_totaal: None };
        let res = bereken_project(&catalogus, &entry);
        assert_eq!(res.zaag_resultaten[0].kosten, 80.0);
    }
}
