'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('bestandsApi', {
  openen: () => ipcRenderer.invoke('bestand:openen'),
  opslaan: (payload) => ipcRenderer.invoke('bestand:opslaan', payload),
  opslaanAls: (payload) => ipcRenderer.invoke('bestand:opslaanAls', payload),
  onMenu: (callback) => ipcRenderer.on('menu:actie', (_e, actie) => callback(actie))
});
