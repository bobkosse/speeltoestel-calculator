'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const Optimizer = require('../src/core/optimizer.js');

test('zaagsnede: 2 x 100 mm past in 2000 mm (203 mm nodig)', () => {
  const bakken = Optimizer.packOptimaal([100, 100], 2000, 3);
  assert.equal(bakken.length, 1);
  assert.equal(bakken[0].rest, 2000 - 100 - 3 - 100);
});

test('zaagsnede: 2 x 1000 mm past NIET in 2000 mm (2003 mm nodig)', () => {
  const bakken = Optimizer.packOptimaal([1000, 1000], 2000, 3);
  assert.equal(bakken.length, 2);
});

test('zaagsnede: 3 x 500 mm past in 2000 mm (1506 mm nodig)', () => {
  const bakken = Optimizer.packOptimaal([500, 500, 500], 2000, 3);
  assert.equal(bakken.length, 1);
  assert.equal(bakken[0].rest, 2000 - 1500 - 2 * 3);
});

test('te lang deel kan niet verpakt worden', () => {
  assert.equal(Optimizer.packOptimaal([2500], 2000, 3), null);
});

test('goedkoopste inkooplengte wordt gekozen (2 x 900 mm)', () => {
  const res = Optimizer.optimaliseerProfiel(
    [900, 900],
    [
      { lengte_mm: 1000, prijs: 10 },
      { lengte_mm: 2000, prijs: 25 }
    ],
    3
  );
  assert.equal(res.lengte_mm, 1000);
  assert.equal(res.aantal, 2);
  assert.equal(res.kosten, 20);
});

test('een 2000 mm plank wint als twee delen samen in 2000 mm passen', () => {
  const res = Optimizer.optimaliseerProfiel(
    [900, 900],
    [
      { lengte_mm: 1000, prijs: 12 },
      { lengte_mm: 2000, prijs: 20 }
    ],
    3
  );
  assert.equal(res.lengte_mm, 2000);
  assert.equal(res.aantal, 1);
  assert.equal(res.kosten, 20);
});

test('zaagsnede telt mee bij keuze tussen lengtes: 2 x 1000 mm -> 2000 mm past niet', () => {
  const res = Optimizer.optimaliseerProfiel(
    [1000, 1000],
    [
      { lengte_mm: 1000, prijs: 12 },
      { lengte_mm: 2000, prijs: 20 }
    ],
    3
  );
  assert.equal(res.lengte_mm, 1000);
  assert.equal(res.aantal, 2);
  assert.equal(res.kosten, 24);
});

test('lege inkooplijst geeft null', () => {
  assert.equal(Optimizer.optimaliseerProfiel([100], [], 3), null);
});

test('berekenProject: per stuk + zaagdeel + btw', () => {
  const data = {
    instellingen: { zaagsnede_mm: 3, btw_pct: 21 },
    per_stuk: [
      { id: 's1', naam: 'Schroef 5x60', prijs: 0.50, info: '' }
    ],
    profielen: [
      {
        id: 'p1', houtsoort: 'Robinia', dikte_mm: 45, breedte_mm: 120,
        inkoop: [{ lengte_mm: 2000, prijs: 40 }]
      }
    ],
    project: [
      { id: 'l1', type: 'per_stuk', materiaal_id: 's1', aantal: 10 },
      { id: 'l2', type: 'zaagdeel', profiel_id: 'p1', lengte_mm: 1000, aantal: 2 }
    ]
  };
  const res = Optimizer.berekenProject(data);
  // 2 x 1000 mm + zaagsnede = 2003 mm -> 2 planken van 2000 mm
  assert.equal(res.zaagResultaten[0].resultaat.aantal, 2);
  assert.equal(res.zaagResultaten[0].kosten, 80);
  assert.equal(res.totaalExcl, 85);
  assert.equal(res.btw, 17.85);
  assert.equal(res.totaalIncl, 102.85);
});

test('berekenProject: zaagdeel dat nowhere past geeft foutmelding', () => {
  const data = {
    instellingen: { zaagsnede_mm: 3, btw_pct: 21 },
    per_stuk: [],
    profielen: [
      { id: 'p1', houtsoort: 'Vuren', dikte_mm: 20, breedte_mm: 100, inkoop: [{ lengte_mm: 1000, prijs: 5 }] }
    ],
    project: [
      { id: 'l1', type: 'zaagdeel', profiel_id: 'p1', lengte_mm: 1500, aantal: 1 }
    ]
  };
  const res = Optimizer.berekenProject(data);
  assert.ok(res.zaagResultaten[0].fout);
  assert.equal(res.totaalExcl, 0);
});
