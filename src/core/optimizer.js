(function (root, factory) {
  if (typeof module === 'object' && module.exports) {
    module.exports = factory();
  } else {
    root.Optimizer = factory();
  }
})(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  var FFD_LIMIET = 40;

  function som(a) {
    var s = 0;
    for (var i = 0; i < a.length; i++) s += a[i];
    return s;
  }

  function pastInBak(lengte, bak, zaagsnede) {
    var nodig = lengte + (bak.stukken.length > 0 ? zaagsnede : 0);
    return nodig <= bak.rest + 1e-9;
  }

  function packFFD(lengtes, stockLengte, zaagsnede) {
    var items = lengtes.slice().sort(function (a, b) { return b - a; });
    var bakken = [];
    for (var i = 0; i < items.length; i++) {
      var geplaatst = false;
      for (var j = 0; j < bakken.length; j++) {
        if (pastInBak(items[i], bakken[j], zaagsnede)) {
          bakken[j].stukken.push(items[i]);
          bakken[j].rest -= items[i] + (bakken[j].stukken.length > 1 ? zaagsnede : 0);
          geplaatst = true;
          break;
        }
      }
      if (!geplaatst) {
        bakken.push({ stukken: [items[i]], rest: stockLengte - items[i] });
      }
    }
    return bakken;
  }

  function packOptimaal(lengtes, stockLengte, zaagsnede) {
    var items = lengtes.slice().sort(function (a, b) { return b - a; });
    if (items.length === 0) return [];
    if (items.length > FFD_LIMIET) return packFFD(lengtes, stockLengte, zaagsnede);

    for (var k = 0; k < items.length; k++) {
      if (items[k] > stockLengte + 1e-9) return null;
    }

    var bakken = [];
    var beste = null;

    function snapshot() {
      return bakken.map(function (b) {
        return { stukken: b.stukken.slice(), rest: b.rest };
      });
    }

    function rec(i) {
      if (beste && bakken.length >= beste.length) return;
      if (i === items.length) {
        if (!beste || bakken.length < beste.length) beste = snapshot();
        return;
      }
      var geprobeerd = {};
      for (var j = 0; j < bakken.length; j++) {
        var sleutel = bakken[j].rest + '|' + bakken[j].stukken.length;
        if (geprobeerd[sleutel]) continue;
        if (pastInBak(items[i], bakken[j], zaagsnede)) {
          geprobeerd[sleutel] = true;
          bakken[j].stukken.push(items[i]);
          bakken[j].rest -= items[i] + (bakken[j].stukken.length > 1 ? zaagsnede : 0);
          rec(i + 1);
          bakken[j].rest += items[i] + (bakken[j].stukken.length > 1 ? zaagsnede : 0);
          bakken[j].stukken.pop();
        }
      }
      bakken.push({ stukken: [items[i]], rest: stockLengte - items[i] });
      rec(i + 1);
      bakken.pop();
    }

    rec(0);
    return beste;
  }

  function optimaliseerProfiel(lengtes, inkoop, zaagsnede) {
    var beste = null;
    for (var i = 0; i < inkoop.length; i++) {
      var optie = inkoop[i];
      var bakken = packOptimaal(lengtes, optie.lengte_mm, zaagsnede);
      if (!bakken) continue;
      var kosten = bakken.length * optie.prijs;
      var afval = bakken.reduce(function (s, b) { return s + b.rest; }, 0);
      if (!beste || kosten < beste.kosten - 1e-9 ||
         (Math.abs(kosten - beste.kosten) <= 1e-9 && afval < beste.afval - 1e-9)) {
        beste = {
          lengte_mm: optie.lengte_mm,
          prijs_per_stuk: optie.prijs,
          aantal: bakken.length,
          bakken: bakken,
          kosten: kosten,
          afval: afval
        };
      }
    }
    return beste;
  }

  function rond2(x) {
    return Math.round(x * 100) / 100;
  }

  function berekenProject(data) {
    var inst = data.instellingen || {};
    var zaagsnede = Number(inst.zaagsnede_mm) || 0;
    var btwPct = Number(inst.btw_pct) || 0;

    var perStukMap = {};
    (data.per_stuk || []).forEach(function (m) { perStukMap[m.id] = m; });
    var profielMap = {};
    (data.profielen || []).forEach(function (p) { profielMap[p.id] = p; });

    var lijnen = [];
    var zaagGroepen = {};

    (data.project || []).forEach(function (lijn) {
      if (lijn.type === 'per_stuk') {
        var mat = perStukMap[lijn.materiaal_id];
        if (!mat) return;
        var aantal = Math.max(0, Number(lijn.aantal) || 0);
        lijnen.push({
          id: lijn.id,
          type: 'per_stuk',
          beschrijving: mat.naam,
          aantal: aantal,
          prijs_per_stuk: mat.prijs,
          kosten: rond2(aantal * mat.prijs)
        });
      } else if (lijn.type === 'zaagdeel') {
        var prof = profielMap[lijn.profiel_id];
        if (!prof) return;
        var lengte = Math.max(0, Number(lijn.lengte_mm) || 0);
        var n = Math.max(0, Math.floor(Number(lijn.aantal) || 0));
        if (!zaagGroepen[prof.id]) {
          zaagGroepen[prof.id] = { profiel: prof, lengtes: [] };
        }
        for (var i = 0; i < n; i++) zaagGroepen[prof.id].lengtes.push(lengte);
        lijnen.push({
          id: lijn.id,
          type: 'zaagdeel',
          beschrijving: prof.naam || (prof.houtsoort + ' ' + prof.dikte_mm + 'x' + prof.breedte_mm + ' mm'),
          lengte_mm: lengte,
          aantal: n
        });
      }
    });

    var zaagResultaten = [];
    var totaal = 0;
    for (var pid in zaagGroepen) {
      var groep = zaagGroepen[pid];
      var res = optimaliseerProfiel(groep.lengtes, groep.profiel.inkoop || [], zaagsnede);
      if (!res) {
        zaagResultaten.push({
          profiel: groep.profiel,
          fout: 'Geen inkooplengte gevonden waarin alle delen passen',
          kosten: 0
        });
        continue;
      }
      totaal += res.kosten;
      zaagResultaten.push({
        profiel: groep.profiel,
        lengtes: groep.lengtes,
        resultaat: res,
        kosten: rond2(res.kosten)
      });
    }

    lijnen.forEach(function (l) { if (l.type === 'per_stuk') totaal += l.kosten; });

    var btw = rond2(totaal * btwPct / 100);
    return {
      zaagsnede_mm: zaagsnede,
      lijnen: lijnen,
      zaagResultaten: zaagResultaten,
      totaalExcl: rond2(totaal),
      btw: btw,
      totaalIncl: rond2(totaal + btw)
    };
  }

  return {
    packOptimaal: packOptimaal,
    packFFD: packFFD,
    optimaliseerProfiel: optimaliseerProfiel,
    berekenProject: berekenProject
  };
});
