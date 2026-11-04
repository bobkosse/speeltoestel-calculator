'use strict';

(function () {
  const Optimizer = window.Optimizer;
  const api = window.bestandsApi;

  function nieuwId() {
    if (window.crypto && crypto.randomUUID) return crypto.randomUUID();
    return 'id-' + Date.now() + '-' + Math.random().toString(36).slice(2);
  }

  const STANDAARD_DATA = {
    versie: 1,
    instellingen: { zaagsnede_mm: 3, btw_pct: 21, munt: 'EUR' },
    per_stuk: [
      { id: nieuwId(), naam: 'Schroef RVS 5 x 60 mm', prijs: 0.12, info: 'diameter 5 mm, lengte 60 mm' },
      { id: nieuwId(), naam: 'Slotbout M8 x 80 mm', prijs: 0.35, info: 'diameter 8 mm, lengte 80 mm, gegalvaniseerd' }
    ],
    profielen: [
      {
        id: nieuwId(),
        houtsoort: 'Robinia',
        dikte_mm: 45,
        breedte_mm: 120,
        inkoop: [{ lengte_mm: 2000, prijs: 48.50 }]
      }
    ],
    project: [
      { id: nieuwId(), type: 'zaagdeel', profiel_id: null, lengte_mm: 1000, aantal: 2 },
      { id: nieuwId(), type: 'per_stuk', materiaal_id: null, aantal: 24 }
    ]
  };
  STANDAARD_DATA.project[0].profiel_id = STANDAARD_DATA.profielen[0].id;
  STANDAARD_DATA.project[1].materiaal_id = STANDAARD_DATA.per_stuk[0].id;

  let data = JSON.parse(JSON.stringify(STANDAARD_DATA));
  let huidigeBestand = null;

  function parseGetal(waarde) {
    const n = parseFloat(String(waarde).replace(',', '.'));
    return Number.isFinite(n) ? n : 0;
  }

  function fmtBedrag(x) {
    const munt = (data.instellingen.munt || 'EUR');
    const symbool = munt === 'EUR' ? '€' : munt + ' ';
    return symbool + Number(x).toLocaleString('nl-NL', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }

  function fmtGetal(x) {
    return Number(x).toLocaleString('nl-NL');
  }

  function profielNaam(p) {
    return p.houtsoort + ' ' + p.dikte_mm + ' x ' + p.breedte_mm + ' mm';
  }

  function esc(s) {
    return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }

  function updateBestandsStatus() {
    document.getElementById('bestandsStatus').textContent =
      huidigeBestand ? 'Bestand: ' + huidigeBestand.split(/[\\/]/).pop() : 'Onopgeslagen project';
    document.title = 'Speeltoestel Calculator' + (huidigeBestand ? ' – ' + huidigeBestand.split(/[\\/]/).pop() : '');
  }

  function renderInstellingen() {
    document.getElementById('instZaagsnede').value = data.instellingen.zaagsnede_mm;
    document.getElementById('instBtw').value = data.instellingen.btw_pct;
    document.getElementById('instMunt').value = data.instellingen.munt;
  }

  function renderPerStuk() {
    const tbody = document.querySelector('#tabelPerStuk tbody');
    tbody.innerHTML = data.per_stuk.map(function (m) {
      return '<tr data-id="' + esc(m.id) + '">' +
        '<td><input class="inp-naam" value="' + esc(m.naam) + '"></td>' +
        '<td><input class="inp-prijs" type="number" min="0" step="0.01" value="' + m.prijs + '"></td>' +
        '<td><input class="inp-info" value="' + esc(m.info || '') + '"></td>' +
        '<td><button class="verwijder" data-actie="verwijder">Verwijder</button></td>' +
        '</tr>';
    }).join('');

    tbody.querySelectorAll('tr').forEach(function (tr) {
      const id = tr.dataset.id;
      const item = data.per_stuk.find(function (m) { return m.id === id; });
      tr.querySelector('.inp-naam').addEventListener('change', function (e) { item.naam = e.target.value; });
      tr.querySelector('.inp-prijs').addEventListener('change', function (e) { item.prijs = parseGetal(e.target.value); });
      tr.querySelector('.inp-info').addEventListener('change', function (e) { item.info = e.target.value; });
      tr.querySelector('[data-actie="verwijder"]').addEventListener('click', function () {
        data.per_stuk = data.per_stuk.filter(function (m) { return m.id !== id; });
        data.project = data.project.filter(function (l) { return l.materiaal_id !== id; });
        renderAll();
      });
    });
  }

  function renderProfielen() {
    const container = document.getElementById('profielLijst');
    container.innerHTML = data.profielen.map(function (p) {
      const inkoopRijen = p.inkoop.map(function (o, idx) {
        return '<tr data-idx="' + idx + '">' +
          '<td><input class="inp-lengte" type="number" min="1" step="1" value="' + o.lengte_mm + '"> mm</td>' +
          '<td><input class="inp-prijs" type="number" min="0" step="0.01" value="' + o.prijs + '"></td>' +
          '<td><button class="verwijder" data-actie="verwijder-inkoop">Verwijder</button></td>' +
          '</tr>';
      }).join('');
      return '<div class="profiel-blok" data-id="' + esc(p.id) + '">' +
        '<div class="profiel-kop">' +
          '<label>Houtsoort <input class="inp-houtsoort" value="' + esc(p.houtsoort) + '"></label>' +
          '<label>Dikte (mm) <input class="inp-dikte" type="number" min="1" value="' + p.dikte_mm + '"></label>' +
          '<label>Breedte (mm) <input class="inp-breedte" type="number" min="1" value="' + p.breedte_mm + '"></label>' +
          '<button class="verwijder" data-actie="verwijder-profiel">Profiel verwijderen</button>' +
        '</div>' +
        '<div class="profiel-inkoop">' +
          '<h3>Inkooplengtes en prijzen (exclusief BTW)</h3>' +
          '<table><thead><tr><th>Lengte</th><th>Prijs per stuk</th><th></th></tr></thead>' +
          '<tbody>' + inkoopRijen + '</tbody></table>' +
          '<div class="veld-rij">' +
            '<input class="nieuw-lengte" type="number" min="1" placeholder="Lengte (mm)">' +
            '<input class="nieuw-prijs" type="number" min="0" step="0.01" placeholder="Prijs">' +
            '<button data-actie="voeg-inkoop-toe">Inkooplengte toevoegen</button>' +
          '</div>' +
        '</div>' +
      '</div>';
    }).join('');

    container.querySelectorAll('.profiel-blok').forEach(function (blok) {
      const id = blok.dataset.id;
      const profiel = data.profielen.find(function (p) { return p.id === id; });
      blok.querySelector('.inp-houtsoort').addEventListener('change', function (e) { profiel.houtsoort = e.target.value; renderCalculatie(); });
      blok.querySelector('.inp-dikte').addEventListener('change', function (e) { profiel.dikte_mm = parseGetal(e.target.value); renderCalculatie(); });
      blok.querySelector('.inp-breedte').addEventListener('change', function (e) { profiel.breedte_mm = parseGetal(e.target.value); renderCalculatie(); });
      blok.querySelector('[data-actie="verwijder-profiel"]').addEventListener('click', function () {
        data.profielen = data.profielen.filter(function (p) { return p.id !== id; });
        data.project = data.project.filter(function (l) { return l.profiel_id !== id; });
        renderAll();
      });
      blok.querySelectorAll('tbody tr').forEach(function (tr) {
        const idx = Number(tr.dataset.idx);
        tr.querySelector('.inp-lengte').addEventListener('change', function (e) { profiel.inkoop[idx].lengte_mm = parseGetal(e.target.value); renderCalculatie(); });
        tr.querySelector('.inp-prijs').addEventListener('change', function (e) { profiel.inkoop[idx].prijs = parseGetal(e.target.value); renderCalculatie(); });
        tr.querySelector('[data-actie="verwijder-inkoop"]').addEventListener('click', function () {
          profiel.inkoop.splice(idx, 1);
          renderAll();
        });
      });
      blok.querySelector('[data-actie="voeg-inkoop-toe"]').addEventListener('click', function () {
        const lengte = parseGetal(blok.querySelector('.nieuw-lengte').value);
        const prijs = parseGetal(blok.querySelector('.nieuw-prijs').value);
        if (lengte > 0) {
          profiel.inkoop.push({ lengte_mm: lengte, prijs: prijs });
          renderAll();
        }
      });
    });
  }

  function renderProject() {
    const tbody = document.querySelector('#tabelProject tbody');
    const perStukOpties = data.per_stuk.map(function (m) {
      return '<option value="' + esc(m.id) + '">' + esc(m.naam) + '</option>';
    }).join('');
    const profielOpties = data.profielen.map(function (p) {
      return '<option value="' + esc(p.id) + '">' + esc(profielNaam(p)) + '</option>';
    }).join('');

    tbody.innerHTML = data.project.map(function (l) {
      let midden;
      if (l.type === 'per_stuk') {
        midden =
          '<td><select class="inp-materiaal"><option value="">– kies –</option>' + perStukOpties + '</select></td>' +
          '<td>–</td>' +
          '<td><input class="inp-aantal" type="number" min="0" step="1" value="' + (l.aantal || 0) + '"></td>';
      } else {
        midden =
          '<td><select class="inp-profiel"><option value="">– kies –</option>' + profielOpties + '</select></td>' +
          '<td><input class="inp-lengte" type="number" min="0" step="1" value="' + (l.lengte_mm || 0) + '"></td>' +
          '<td><input class="inp-aantal" type="number" min="0" step="1" value="' + (l.aantal || 0) + '"></td>';
      }
      return '<tr data-id="' + esc(l.id) + '" data-type="' + l.type + '">' +
        '<td><select class="inp-type">' +
          '<option value="zaagdeel"' + (l.type === 'zaagdeel' ? ' selected' : '') + '>Zaagdeel</option>' +
          '<option value="per_stuk"' + (l.type === 'per_stuk' ? ' selected' : '') + '>Per stuk</option>' +
        '</select></td>' +
        midden +
        '<td><button class="verwijder" data-actie="verwijder">Verwijder</button></td>' +
      '</tr>';
    }).join('');

    tbody.querySelectorAll('tr').forEach(function (tr) {
      const id = tr.dataset.id;
      const lijn = data.project.find(function (l) { return l.id === id; });
      tr.querySelector('.inp-type').addEventListener('change', function (e) {
        lijn.type = e.target.value;
        if (lijn.type === 'per_stuk') { lijn.materiaal_id = data.per_stuk[0] ? data.per_stuk[0].id : null; lijn.lengte_mm = 0; }
        else { lijn.profiel_id = data.profielen[0] ? data.profielen[0].id : null; }
        renderAll();
      });
      const matSel = tr.querySelector('.inp-materiaal');
      if (matSel) {
        matSel.value = lijn.materiaal_id || '';
        matSel.addEventListener('change', function (e) { lijn.materiaal_id = e.target.value || null; renderCalculatie(); });
      }
      const profSel = tr.querySelector('.inp-profiel');
      if (profSel) {
        profSel.value = lijn.profiel_id || '';
        profSel.addEventListener('change', function (e) { lijn.profiel_id = e.target.value || null; renderCalculatie(); });
      }
      const lenInp = tr.querySelector('.inp-lengte');
      if (lenInp) lenInp.addEventListener('change', function (e) { lijn.lengte_mm = parseGetal(e.target.value); renderCalculatie(); });
      tr.querySelector('.inp-aantal').addEventListener('change', function (e) { lijn.aantal = parseGetal(e.target.value); renderCalculatie(); });
      tr.querySelector('[data-actie="verwijder"]').addEventListener('click', function () {
        data.project = data.project.filter(function (l) { return l.id !== id; });
        renderAll();
      });
    });
  }

  function renderCalculatie() {
    const container = document.getElementById('calculatieInhoud');
    const res = Optimizer.berekenProject(data);
    const zaagsnede = res.zaagsnede_mm;

    let html = '<p class="uitleg">Zaagsnede per snede: <strong>' + fmtGetal(zaagsnede) + ' mm</strong> – bij n delen uit één plank zijn er n−1 zaagsnedes nodig.</p>';

    if (res.zaagResultaten.length > 0) {
      html += '<h3>Zaagdelen per profiel</h3>';
      res.zaagResultaten.forEach(function (zr) {
        const p = zr.profiel;
        html += '<div class="zaagplan"><h3>' + esc(profielNaam(p)) + '</h3>';
        if (zr.fout) {
          html += '<p class="fout">' + esc(zr.fout) + '</p>';
        } else {
          const r = zr.resultaat;
          html += '<p>Inkoop: <strong>' + r.aantal + ' × ' + fmtGetal(r.lengte_mm) + ' mm</strong> à ' + fmtBedrag(r.prijs_per_stuk) +
            ' = <strong>' + fmtBedrag(zr.kosten) + '</strong> (restafval totaal: ' + fmtGetal(Math.round(r.afval)) + ' mm)</p>';
          html += '<ul>';
          r.bakken.forEach(function (bak, i) {
            html += '<li>Plank ' + (i + 1) + ' (' + fmtGetal(r.lengte_mm) + ' mm): ' +
              bak.stukken.map(function (s) { return fmtGetal(s) + ' mm'; }).join(' + ') +
              ' <em>(zaagsnedes: ' + (bak.stukken.length - 1) + ' × ' + fmtGetal(zaagsnede) + ' mm)</em>' +
              ' → rest ' + fmtGetal(Math.round(bak.rest)) + ' mm</li>';
          });
          html += '</ul>';
        }
        html += '</div>';
      });
    }

    const perStukLijnen = res.lijnen.filter(function (l) { return l.type === 'per_stuk'; });
    if (perStukLijnen.length > 0) {
      html += '<h3>Producten per stuk</h3><table><thead><tr><th>Naam</th><th>Aantal</th><th>Prijs/stuk</th><th>Kosten</th></tr></thead><tbody>';
      perStukLijnen.forEach(function (l) {
        html += '<tr><td>' + esc(l.beschrijving) + '</td><td>' + fmtGetal(l.aantal) + '</td><td>' + fmtBedrag(l.prijs_per_stuk) + '</td><td>' + fmtBedrag(l.kosten) + '</td></tr>';
      });
      html += '</tbody></table>';
    }

    html += '<div class="totaal-kaart">' +
      '<div class="totaal-rij"><span>Totaal exclusief BTW</span><span>' + fmtBedrag(res.totaalExcl) + '</span></div>' +
      '<div class="totaal-rij"><span>BTW (' + fmtGetal(data.instellingen.btw_pct || 0) + '%)</span><span>' + fmtBedrag(res.btw) + '</span></div>' +
      '<div class="totaal-rij groot"><span>Totaal inclusief BTW</span><span>' + fmtBedrag(res.totaalIncl) + '</span></div>' +
      '</div>';

    container.innerHTML = html;
  }

  function renderAll() {
    renderInstellingen();
    renderPerStuk();
    renderProfielen();
    renderProject();
    renderCalculatie();
    updateBestandsStatus();
  }

  function voegPerStukToe() {
    const naam = document.getElementById('nieuwPerStukNaam').value.trim();
    const prijs = parseGetal(document.getElementById('nieuwPerStukPrijs').value);
    const info = document.getElementById('nieuwPerStukInfo').value.trim();
    if (!naam) return;
    data.per_stuk.push({ id: nieuwId(), naam: naam, prijs: prijs, info: info });
    document.getElementById('nieuwPerStukNaam').value = '';
    document.getElementById('nieuwPerStukPrijs').value = '';
    document.getElementById('nieuwPerStukInfo').value = '';
    renderAll();
  }

  function voegProfielToe() {
    data.profielen.push({
      id: nieuwId(),
      houtsoort: 'Houtsoort',
      dikte_mm: 45,
      breedte_mm: 120,
      inkoop: [{ lengte_mm: 2000, prijs: 0 }]
    });
    renderAll();
  }

  function voegProjectRijToe(type) {
    const rij = { id: nieuwId(), type: type, aantal: 1, lengte_mm: 0, materiaal_id: null, profiel_id: null };
    if (type === 'per_stuk' && data.per_stuk[0]) rij.materiaal_id = data.per_stuk[0].id;
    if (type === 'zaagdeel' && data.profielen[0]) rij.profiel_id = data.profielen[0].id;
    data.project.push(rij);
    renderAll();
  }

  async function bestandOpenen() {
    const r = await api.openen();
    if (!r) return;
    try {
      const geparseerd = JSON.parse(r.data);
      if (!geparseerd || !Array.isArray(geparseerd.per_stuk) || !Array.isArray(geparseerd.profielen) || !Array.isArray(geparseerd.project)) {
        throw new Error('Ongeldig bestand');
      }
      data = geparseerd;
      data.instellingen = data.instellingen || { zaagsnede_mm: 3, btw_pct: 21, munt: 'EUR' };
      huidigeBestand = r.path;
      renderAll();
    } catch (err) {
      alert('Kon bestand niet lezen: ' + err.message);
    }
  }

  async function bestandOpslaan(alsAnders) {
    const payload = { data: JSON.stringify(data, null, 2) };
    let r;
    if (!alsAnders && huidigeBestand) {
      r = await api.opslaan({ path: huidigeBestand, data: payload.data });
    } else {
      r = await api.opslaanAls(payload);
    }
    if (r && r.path) {
      huidigeBestand = r.path;
      updateBestandsStatus();
    }
  }

  function init() {
    document.querySelectorAll('.tab').forEach(function (btn) {
      btn.addEventListener('click', function () {
        document.querySelectorAll('.tab').forEach(function (b) { b.classList.remove('actief'); });
        document.querySelectorAll('.tabblad').forEach(function (s) { s.classList.remove('actief'); });
        btn.classList.add('actief');
        document.getElementById('tab-' + btn.dataset.tab).classList.add('actief');
      });
    });

    document.getElementById('instZaagsnede').addEventListener('change', function (e) {
      data.instellingen.zaagsnede_mm = parseGetal(e.target.value);
      renderCalculatie();
    });
    document.getElementById('instBtw').addEventListener('change', function (e) {
      data.instellingen.btw_pct = parseGetal(e.target.value);
      renderCalculatie();
    });
    document.getElementById('instMunt').addEventListener('change', function (e) {
      data.instellingen.munt = e.target.value.trim() || 'EUR';
      renderCalculatie();
    });

    document.getElementById('btnVoegPerStukToe').addEventListener('click', voegPerStukToe);
    document.getElementById('btnVoegProfielToe').addEventListener('click', voegProfielToe);
    document.getElementById('btnVoegPerStukRijToe').addEventListener('click', function () { voegProjectRijToe('per_stuk'); });
    document.getElementById('btnVoegZaagdeelRijToe').addEventListener('click', function () { voegProjectRijToe('zaagdeel'); });

    document.getElementById('btnNieuw').addEventListener('click', function () {
      if (confirm('Nieuw project starten? Niet-opgeslagen wijzigingen gaan verloren.')) {
        data = JSON.parse(JSON.stringify(STANDAARD_DATA));
        data.per_stuk.forEach(function (m) { m.id = nieuwId(); });
        data.profielen.forEach(function (p) { p.id = nieuwId(); });
        data.project = [];
        huidigeBestand = null;
        renderAll();
      }
    });
    document.getElementById('btnOpenen').addEventListener('click', bestandOpenen);
    document.getElementById('btnOpslaan').addEventListener('click', function () { bestandOpslaan(false); });
    document.getElementById('btnOpslaanAls').addEventListener('click', function () { bestandOpslaan(true); });

    if (api && api.onMenu) {
      api.onMenu(function (actie) {
        if (actie === 'nieuw') document.getElementById('btnNieuw').click();
        if (actie === 'openen') bestandOpenen();
        if (actie === 'opslaan') bestandOpslaan(false);
        if (actie === 'opslaanAls') bestandOpslaan(true);
      });
    }

    renderAll();
  }

  document.addEventListener('DOMContentLoaded', init);
})();
