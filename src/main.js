'use strict';

const { app, BrowserWindow, Menu, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs');

let win = null;

const jsonFilter = [{ name: 'JSON-bestand', extensions: ['json'] }];

function createWindow() {
  win = new BrowserWindow({
    width: 1280,
    height: 860,
    minWidth: 980,
    minHeight: 640,
    title: 'Speeltoestel Calculator',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  });
  win.loadFile(path.join(__dirname, 'renderer', 'index.html'));
}

function menuActie(actie) {
  if (win) win.webContents.send('menu:actie', actie);
}

function bouwMenu() {
  const template = [
    {
      label: 'Bestand',
      submenu: [
        { label: 'Nieuw project', accelerator: 'CmdOrCtrl+N', click: () => menuActie('nieuw') },
        { label: 'Openen…', accelerator: 'CmdOrCtrl+O', click: () => menuActie('openen') },
        { label: 'Opslaan', accelerator: 'CmdOrCtrl+S', click: () => menuActie('opslaan') },
        { label: 'Opslaan als…', accelerator: 'CmdOrCtrl+Shift+S', click: () => menuActie('opslaanAls') },
        { type: 'separator' },
        { role: 'quit', label: 'Afsluiten' }
      ]
    },
    {
      label: 'Bewerken',
      submenu: [
        { role: 'undo', label: 'Ongedaan maken' },
        { role: 'redo', label: 'Opnieuw' },
        { type: 'separator' },
        { role: 'cut', label: 'Knippen' },
        { role: 'copy', label: 'Kopiëren' },
        { role: 'paste', label: 'Plakken' }
      ]
    }
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

ipcMain.handle('bestand:openen', async () => {
  const r = await dialog.showOpenDialog(win, {
    properties: ['openFile'],
    filters: jsonFilter
  });
  if (r.canceled || r.filePaths.length === 0) return null;
  const bestandspad = r.filePaths[0];
  const data = fs.readFileSync(bestandspad, 'utf8');
  return { path: bestandspad, data };
});

ipcMain.handle('bestand:opslaan', async (_e, payload) => {
  fs.writeFileSync(payload.path, payload.data, 'utf8');
  return { path: payload.path };
});

ipcMain.handle('bestand:opslaanAls', async (_e, payload) => {
  const r = await dialog.showSaveDialog(win, {
    defaultPath: 'speeltoestel-project.json',
    filters: jsonFilter
  });
  if (r.canceled || !r.filePath) return null;
  fs.writeFileSync(r.filePath, payload.data, 'utf8');
  return { path: r.filePath };
});

app.whenReady().then(() => {
  bouwMenu();
  createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
